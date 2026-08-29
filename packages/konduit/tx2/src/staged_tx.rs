use std::collections::{BTreeMap, BTreeSet};

use cardano_sdk::{
    Address, ChangeStrategy, Hash, Input, NetworkId, Output, SlotBound, Transaction, Value,
    VerificationKey, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{Duration, Redeemer, Step};
use konduit_tmp::from_verifying_key;

use crate::{
    Channel, FEE_BUFFER, Interval, NetworkParameters,
    channel::FromOutputError,
    fuel, konduit_address,
    step::{Want, Will},
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("no channel for this input")]
    MissingInput,
    #[error(transparent)]
    Channel(#[from] FromOutputError),
    #[error(transparent)]
    Step(#[from] crate::step::Error),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum BuildError {
    #[error("reference script required when stepping channels")]
    MissingReference,
    #[error("no utxo on record for a willed input")]
    MissingUtxo,
    #[error(transparent)]
    Fuel(#[from] fuel::SelectError),
    #[error("cannot balance tx")]
    Balancing,
}

/// The "Stage" artifact of the pipeline:
///   scan chain -> parse state -> filter relevant -> Stage -> choose steps -> resolve -> build
pub struct StagedTx {
    network_id: NetworkId,
    interval: Interval,
    channels: BTreeMap<Input, Channel>,
    wills: BTreeMap<Input, Will>,
    opens: BTreeSet<Channel>,
}

impl StagedTx {
    pub fn new(
        network_id: NetworkId,
        interval: Interval,
        channels: BTreeMap<Input, Channel>,
    ) -> Self {
        Self {
            network_id,
            interval,
            channels,
            wills: BTreeMap::new(),
            opens: BTreeSet::new(),
        }
    }

    pub fn channels(&self) -> &BTreeMap<Input, Channel> {
        &self.channels
    }

    // --- Wills ---

    /// Registers a `Want` for `input` against its parsed pre-step `Channel`.
    /// Overwrites any existing intent already cached for the same input
    /// (last-write-wins — lets an interactive caller freely change their
    /// mind before building).
    pub fn want(&mut self, input: Input, want: Want) -> Result<(), Error> {
        let channel = self.channels.get(&input).ok_or(Error::MissingInput)?;
        let will = channel.resolve(want, &self.interval)?;
        self.wills.insert(input, will);
        Ok(())
    }

    pub fn drop_intent(&mut self, input: &Input) -> Option<Will> {
        self.wills.remove(input)
    }

    pub fn drop_all_intents(&mut self) {
        self.wills.clear();
    }

    // --- Opens ---

    pub fn opens(&self) -> &BTreeSet<Channel> {
        &self.opens
    }

    pub fn add_open(&mut self, channel: Channel) -> bool {
        self.opens.insert(channel)
    }

    pub fn retain(&mut self, pred: impl FnMut(&Channel) -> bool) {
        self.opens.retain(pred)
    }

    pub fn drop_all_opens(&mut self) {
        self.opens.clear();
    }

    // --- Inspect before build ---

    pub fn gain(&self) -> i64 {
        self.wills
            .iter()
            .filter_map(|(input, will)| {
                let pre_amount = self.channels.get(input)?.amount() as i64;
                let cont_amount = match will {
                    Will::Cont { output, .. } => output.amount() as i64,
                    Will::Eol { .. } => 0,
                };
                Some(pre_amount - cont_amount)
            })
            .sum()
    }

    pub fn signers(&self) -> Vec<VerificationKey> {
        let mut signers: Vec<_> = self
            .wills
            .iter()
            .filter_map(|(input, will)| {
                let constants = self.channels.get(input)?.constants();
                Some(from_verifying_key(will.signer(constants)))
            })
            .collect();
        signers.sort();
        signers.dedup();
        signers
    }

    fn channel_output(&self, channel: &Channel) -> Output {
        Output::new(
            konduit_address(self.network_id, channel.delegation().as_ref()).into(),
            channel.buffered_value(),
        )
        .with_datum({
            let bytes = minicbor::to_vec(channel.datum()).expect("encode");
            minicbor::decode(&bytes).expect("valid cbor round-trips into PlutusData")
        })
    }

    pub fn outputs(&self) -> Vec<Output> {
        let cont = self.wills.values().filter_map(|will| match will {
            Will::Cont {
                output: channel, ..
            } => Some(self.channel_output(channel)),
            Will::Eol { .. } => None,
        });
        let opens = self
            .opens
            .iter()
            .map(|channel| self.channel_output(channel));
        cont.chain(opens).collect()
    }

    pub fn inputs(&self) -> Vec<(Input, Redeemer)> {
        self.wills
            .keys()
            .enumerate()
            .map(|(i, input)| (input.clone(), self.redeemer(i)))
            .collect()
    }

    fn redeemer(&self, index: usize) -> Redeemer {
        if index == 0 {
            Redeemer::Main(self.steps())
        } else {
            Redeemer::Defer
        }
    }

    pub fn steps(&self) -> Vec<Step> {
        self.wills.values().map(Will::to_step).collect()
    }

    // --- Build ---

    pub fn build(
        &mut self,
        utxos: &BTreeMap<Input, Output>,
        network_parameters: &NetworkParameters,
        reference_utxo: Option<&(Input, Output)>,
        change_address: Address<kind::Any>,
        fuel: &BTreeMap<Input, Output>,
    ) -> Result<Transaction<ReadyForSigning>, BuildError> {
        let reference_inputs: Vec<_> = reference_utxo.iter().map(|x| x.0.clone()).collect();
        if !self.wills.is_empty() && reference_inputs.is_empty() {
            return Err(BuildError::MissingReference);
        }

        let mut spent_value = Value::new(0);
        for input in self.wills.keys() {
            let output = utxos.get(input).ok_or(BuildError::MissingUtxo)?;
            spent_value.add(output.value());
        }

        let outputs = self.outputs();
        let produced_value = outputs.iter().fold(Value::new(0), |mut acc, output| {
            acc.add(output.value());
            acc
        });

        // Shortfall: whatever `produced` needs beyond `spent`, floored at 0.
        let mut target = fuel::saturating_sub_value(&produced_value, &spent_value);
        let target_lovelace = target.lovelace() + FEE_BUFFER;
        target.with_lovelace(target_lovelace);
        let fuel_inputs = fuel::select(fuel, &target)?;

        let inputs: Vec<_> = self
            .inputs()
            .into_iter()
            .map(|(input, redeemer)| {
                (
                    input,
                    Some({
                        let bytes = minicbor::to_vec(redeemer).expect("encode");
                        minicbor::decode(&bytes).expect("valid cbor round-trips into PlutusData")
                    }),
                )
            })
            .chain(fuel_inputs.iter().map(|i| (i.clone(), None)))
            .collect();

        let collaterals = fuel_inputs.clone();
        let specified_signatories = self
            .signers()
            .iter()
            .map(Hash::<28>::new)
            .collect::<Vec<_>>();

        let to_slot = |d: Duration| network_parameters.protocol_parameters.posix_to_slot(*d);
        let lower_bound = self
            .interval
            .lower
            .map_or(SlotBound::None, |d| SlotBound::Inclusive(to_slot(d)));
        let upper_bound = self
            .interval
            .upper
            .map_or(SlotBound::None, |d| SlotBound::Exclusive(to_slot(d)));

        let tx_utxos = utxos
            .iter()
            .chain(fuel.iter())
            .map(|(i, o)| (i.clone(), o.clone()))
            .chain(reference_utxo.iter().map(|i| (i.0.clone(), i.1.clone())))
            .collect::<BTreeMap<_, _>>();

        let tx = Transaction::build(
            &network_parameters.protocol_parameters,
            &tx_utxos,
            |transaction| {
                transaction
                    .with_inputs(inputs.clone())
                    .with_collaterals(collaterals.clone())
                    .with_reference_inputs(reference_inputs.clone())
                    .with_outputs(outputs.clone())
                    .with_specified_signatories(specified_signatories.clone())
                    .with_validity_interval(lower_bound, upper_bound)
                    .with_change_strategy(ChangeStrategy::as_last_output(change_address.clone()))
                    .ok()
            },
        )
        .map_err(|_err| BuildError::Balancing)?;

        self.wills.clear();
        self.opens.clear();

        Ok(tx)
    }
}
