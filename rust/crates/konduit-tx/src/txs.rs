use std::cmp::min;
use std::collections::{BTreeMap, BTreeSet};

use anyhow::anyhow;
use cardano_tx_builder as cardano;
use cardano_tx_builder::{
    Address, ChangeStrategy, Credential, Hash, Input, NetworkId, Output, PlutusData, PlutusScript,
    ProtocolParameters, Transaction, Value, VerificationKey, address,
    transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Datum, Duration, Receipt, Redeemer, Stage, Tag};

pub type Lovelace = u64;

pub const MIN_ADA: Lovelace = 2_000_000;

pub const FEE_BUFFER: Lovelace = 3_000_000;

pub type Utxos = BTreeMap<Input, Output>;

pub type Utxo = (Input, Output);

pub fn deploy(
    protocol_parameters: &ProtocolParameters,
    utxos: &Utxos,
    script: PlutusScript,
    host_address: Address<address::kind::Any>,
    change_address: Address<address::kind::Any>,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let outputs = vec![Output::to(host_address).with_plutus_script(script)];

    let inputs = utxos.iter().map(|(input, _)| (input.clone(), None));
    Transaction::build(&protocol_parameters, &utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .ok()
    })
}

pub fn select_utxos(utxos: &Utxos, amount: Lovelace) -> anyhow::Result<Vec<&Input>> {
    // Filter out utxos which hold reference scripts
    let mut sorted_utxos: Vec<(&Input, &Output)> = utxos
        .into_iter()
        .filter(|(_, output)| output.script().is_none())
        .collect();
    sorted_utxos.sort_by_key(|(_, output)| std::cmp::Reverse(output.value().lovelace()));

    let mut selected_inputs = Vec::new();
    let mut total_lovelace: u64 = 0;

    for (input, output) in sorted_utxos {
        selected_inputs.push(input);
        total_lovelace = total_lovelace.saturating_add(output.value().lovelace());

        if total_lovelace >= amount {
            break;
        }
    }
    if total_lovelace < amount {
        return Err(anyhow!("insufficient funds in wallet to cover the amount"));
    }
    Ok(selected_inputs)
}

pub fn open(
    // A backend to Cardano
    // connector: impl CardanoConnect,
    utxos: &BTreeMap<Input, Output>,
    protocol_parameters: &ProtocolParameters,
    network_id: NetworkId,
    // Konduit's validator hash,
    validator: Hash<28>,
    // Quantity of Lovelace to deposit into the channel
    amount: Lovelace,
    // Consumer's verification key, allowed to *add* funds.
    consumer: VerificationKey,
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: VerificationKey,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: Tag,
    // Minimum time from `close` to `elapse`.
    close_period: Duration,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let consumer_payment_credential = Credential::from_key(Hash::<28>::new(consumer));
    let consumer_staking_credential = Credential::from_key(Hash::<28>::new(consumer));
    let contract_address = Address::from(
        Address::new(network_id, Credential::from_script(validator))
            .with_delegation(consumer_staking_credential.clone()),
    );
    let consumer_change_address = Address::from(
        Address::new(network_id, consumer_payment_credential)
            .with_delegation(consumer_staking_credential),
    );

    let datum = PlutusData::from(Datum {
        own_hash: validator,
        constants: Constants {
            tag,
            add_vkey: consumer,
            sub_vkey: adaptor,
            close_period,
        },
        stage: Stage::Opened(amount),
    });

    let funding_inputs: Vec<&Input> = select_utxos(utxos, amount + FEE_BUFFER)?;

    Transaction::build(&protocol_parameters, &utxos, |transaction| {
        transaction
            .with_inputs(funding_inputs.iter().map(|&i| (i.clone(), None)))
            .with_outputs([
                Output::new(contract_address.clone(), Value::new(amount)).with_datum(datum.clone())
            ])
            .with_change_strategy(ChangeStrategy::as_last_output(
                consumer_change_address.clone(),
            ))
            .ok()
    })
}

// pub type Step {
//   StepCont(Cont)
//   StepEol(Eol)
// }
//
// pub type Unpend =
//   ByteArray
//
// pub type Cont {
//   Add
//   Sub(Squash, List<Unlocked>)
//   Close
//   Respond(Squash, List<MixedCheque>)
//   Unlock(List<Unpend>)
//   Expire(List<Unpend>)
// }
//
// pub type Redeemer {
//   Defer
//   Main(List<Step>)
//   Mutual
// }
//
pub fn parse_output_datum(channel_output: &Output) -> anyhow::Result<konduit_data::Datum> {
    let datum_rc = channel_output
        .datum()
        .ok_or(anyhow!("Output has no datum"))?;
    match datum_rc {
        cardano::Datum::Inline(plutus_data) => {
            let konduit_datum = konduit_data::Datum::try_from(plutus_data.clone())
                .map_err(|e| anyhow!("Failed to convert output datum to konduit datum: {:?}", e))?;
            Ok(konduit_datum)
        }
        cardano::Datum::Hash(_) => Err(anyhow!("Datum is a hash, expected inline datum")),
    }
}

pub fn mk_sub_step(receipt: &Receipt, channel_in: &Output) -> Option<(Lovelace, Output)> {
    let datum_in: Datum = parse_output_datum(channel_in).ok()?;
    let value_in: Value<u64> = channel_in.value().clone();
    let available = value_in.lovelace() - MIN_ADA;
    if let Stage::Opened(subbed_in) = datum_in.stage {
        let to_sub = {
            let owed = receipt.amount();
            min(available, owed - subbed_in)
        };
        let stage_out = Stage::Opened(subbed_in + to_sub);
        let datum_out = Datum {
            own_hash: datum_in.own_hash,
            constants: datum_in.constants,
            stage: stage_out,
        };
        // L1 is permissive on the input value and extra tokens but
        // it is restrictive on the output value which must be pure ADA.
        let value_out = Value::new(value_in.lovelace() - to_sub);
        let channel_out = Output::new(channel_in.address().clone(), value_out)
            .with_datum(PlutusData::from(datum_out));
        if let Some(plutus_script) = channel_in.script() {
            Some((
                to_sub,
                channel_out.with_plutus_script(plutus_script.clone()),
            ))
        } else {
            let channel_out = channel_out;
            Some((to_sub, channel_out))
        }
    } else {
        None
    }
}

fn output_signatory(output: &Output) -> Option<Hash<28>> {
    output
        .address()
        .as_shelley()
        .map(|addr| addr.payment())
        .and_then(|payment_credential| payment_credential.as_key())
}

// Used by a pure sub transaction
pub const SUB_THRESHOLD: Lovelace = 0;

pub fn sub(
    // Utxos providing the necessary funds
    funding_utxos: &BTreeMap<Input, Output>,
    // Utxo holding the konduit script
    script_utxo: &Utxo,
    // Utxos representing the channels to be subtracted from
    channels_in: &Vec<Utxo>,
    // Receipt which authorizes the subtraction
    receipt: &konduit_data::Receipt,
    // Adaptor's verification key, allowed to *sub* funds
    // For now we use it as a baseline for the change and cash out address
    // We use that key for both parts of the change address.
    adaptor: VerificationKey,
    // Network information: params and id
    protocol_parameters: &ProtocolParameters,
    network_id: NetworkId,
) -> anyhow::Result<Option<Transaction<ReadyForSigning>>> {
    let tx_steps = channels_in
        .iter()
        .filter_map(|(txout_ref, channel_in)| {
            mk_sub_step(receipt, channel_in)
                .map(|(to_sub, channel_out)| (to_sub, txout_ref, channel_out))
        })
        .collect::<Vec<(Lovelace, &Input, Output)>>();

    let to_sub = tx_steps
        .iter()
        .map(|&(to_sub, _, _)| to_sub)
        .sum::<Lovelace>();

    // if there is nothing to sub from, return None
    if to_sub <= SUB_THRESHOLD {
        return Ok(None);
    }

    let mut tx_steps: Vec<(&Input, Output)> = tx_steps
        .into_iter()
        .map(|(_, txout_ref, channel_out)| (txout_ref, channel_out))
        .collect::<Vec<(&Input, Output)>>();
    tx_steps.sort_by_key(|&(txout_ref, _)| txout_ref.clone());

    let main_redeemer: Redeemer = {
        let cont = konduit_data::Cont::Sub(receipt.squash.clone(), receipt.unlockeds.clone());
        let sub = konduit_data::Step::Cont(cont);
        let steps = vec![sub.clone(); tx_steps.len()];
        konduit_data::Redeemer::Main(steps)
    };
    let defer = konduit_data::Redeemer::Defer;

    let inputs: Vec<(&Input, Option<PlutusData>)> = {
        let channel_inputs: Vec<(&Input, _)> = tx_steps
            .iter()
            .enumerate()
            .map(|(idx, &(txout_ref, _))| {
                if idx == 0 {
                    (txout_ref, Some(PlutusData::from(main_redeemer.clone())))
                } else {
                    (txout_ref, Some(PlutusData::from(defer.clone())))
                }
            })
            .collect::<Vec<(&Input, Option<PlutusData>)>>();
        let selected_utxos = select_utxos(&funding_utxos, FEE_BUFFER - to_sub)?;
        let funding_inputs = selected_utxos
            .iter()
            .map(|&i| (i, None))
            .collect::<Vec<_>>();
        [channel_inputs, funding_inputs].concat()
    };

    let outputs = tx_steps
        .iter()
        .map(|(_, channel_out)| channel_out.clone())
        .collect::<Vec<Output>>();

    let utxos: BTreeMap<Input, Output> = {
        let mut base = funding_utxos.clone();
        // add script utxo
        base.insert(script_utxo.0.clone(), script_utxo.1.clone());
        for &(txout_ref, ref channel_out) in tx_steps.iter() {
            base.insert(txout_ref.clone(), channel_out.clone());
        }
        base
    };

    let change_address = Address::from(
        Address::new(
            network_id,
            Credential::from_key(Hash::<28>::new(adaptor.clone())),
        )
        .with_delegation(Credential::from_key(Hash::<28>::new(adaptor.clone()))),
    );

    let specified_signatories = {
        // signatories implied by the inputs
        let ledger_signatories = inputs
            .iter()
            .filter_map(|(input, _)| utxos.get(input).and_then(|output| output_signatory(output)))
            .collect::<BTreeSet<Hash<28>>>();
        let key_hash = Hash::<28>::new(adaptor.clone());
        // Check if the adaptor is among them, if it is not then we have to specify it explicitly
        if !ledger_signatories.contains(&key_hash) {
            vec![key_hash]
        } else {
            vec![]
        }
    };
    let inputs = inputs
        .into_iter()
        .map(|(i, redeemer)| (i.clone(), redeemer))
        .collect::<Vec<(Input, Option<PlutusData>)>>();

    let transaction = Transaction::build(&protocol_parameters, &utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_reference_inputs(vec![script_utxo.0.clone()])
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .with_specified_signatories(specified_signatories.to_owned())
            .ok()
    })?;
    Ok(Some(transaction))
}

// pub struct TxStep<'a> {
//     input: Utxo,
//     output: Option<Output>,
//     redeemer: PlutusData<'a>,
// }
//
// pub struct TxSteps<'a> {
//     steps: Vec<TxStep<'a>>,
//     signatories: BTreeSet<VerificationKey>,
// }
//
// // inputs has to be sorted and payout should be added
// pub fn mappend_txsteps<'a>(tx1: TxSteps<'a>, tx2: TxSteps<'a>) -> TxSteps<'a> {
//     let mut steps = tx1.steps;
//     steps.extend(tx2.steps);
//     steps.sort_by_key(|step| step.input.0.clone());
//
//     let mut signatories = tx1.signatories;
//     signatories.extend(tx2.signatories);
//     TxSteps { steps, signatories }
// }
//
// // Singleton steps tx
// pub fn mk_tx_steps(
//     input: Utxo,
//     output: Option<Output>,
//     redeemer: PlutusData,
//     signatory: VerificationKey,
// ) -> TxSteps {
//     let step = TxStep {
//         input,
//         output,
//         redeemer,
//     };
//     let mut signatories = BTreeSet::new();
//     signatories.insert(signatory);
//     TxSteps {
//         steps: vec![step],
//         signatories,
//     }
// }
//
// // pub fn with_validity_interval(&mut self, from: SlotBound, until: SlotBound) -> &mut Self {
//
// // Simplified Konduit specific transaction input/output:
// // * We keep inputs around so we can shuffle them as we go.
// // * We preserve redeemer in the `Intent` form so we can
// //  build the final redeemer(s) in a structured way at the end.
// // * We ignore here Konduit script reference input - it will be added later.
// // * We do not account for change outputs here - they will be added by the tx builder.
// enum InputOutput<'a> {
//     // Either funding input or eol script step.
//     InputOnly(Input, Option<Intent>),
//     // New channel output - change output will be added later.
//     OutputOnly(Output),
//     InputOutput(Input, Intent, Output),
// }
// // We want to have ord:
// // `OutputOnly` is
// // impl
//
// struct TxBody<'a> {
//     // We do not preserve order here. It is executed once at the end
//     inputs_outputs: Vec<InputOutput<'a>>,
//     // signatories are stripped away from here
//     extra_signatories: BTreeSet<VerificationKey>,
//     // Regular spend input implied signatories
//     signatories: BTreeSet<VerificationKey>,
//     // `inputs - outputs` for rough founding requirements
//     balance: Lovelace,
//
// }
//
// impl TxBody<'a> {
//     fn new() -> Self {
//         TxBody {
//             inputs_outputs: Vec::new(),
//             signatories: BTreeSet::new(),
//             extra_signatories: BTreeSet::new(),
//             balance: 0
//         }
//     }
//
//     // pub struct Address<T: IsAddressKind>(Arc<AddressKind>, PhantomData<T>);
//     // enum AddressKind {
//     //     Byron(pallas::ByronAddress),
//     //     Shelley(pallas::ShelleyAddress),
//     // }
//     // pub enum ShelleyPaymentPart {
//     //     Key(PaymentKeyHash),
//     //     Script(ScriptHash),
//     // }
//     //
//     // pub fn as_shelley(&self) -> Option<Address<kind::Shelley>> {
//     //     if self.is_shelley() {
//     //         return Some(Address(self.0.clone(), PhantomData));
//     //     }
//
//     //     None
//     // }
//     // Address<Shelley>
//     // pub fn payment(&self) -> Credential {
//     //     Credential::from(self.cast().payment())
//     // }
//     // Credential::pub fn as_key(&self) -> Option<Hash<28>> {
//     //     self.select(Some, |_| None)
//     // }
//     fn with_input(
//         &mut self,
//         utxo: &Utxo,
//         // redeemer and extra signatory - all the actions
//         // which we have require a signatory
//         script_extras: Option<(PlutusData<'_>, VerificationKey)>,
//     ) -> {
//         let (input_ref, input) = utxo;
//         let input_only = match script_extras {
//             Some((redeemer, extra_signatory)) => {
//                 // If extra_signatory is not present in the sets we add it to the extra_signatories
//                 if(!self.signatories.contains(&extra_signatory) && !self.extra_signatories.contains(&extra_signatory)) {
//                     self.extra_signatories.insert(extra_signatory);
//                 }
//                 self.balance = self.balance + input.value().lovelace();
//                 InputOnly(input_ref.clone(), Some(redeemer.clone()))
//
//             }
//             None => {
//                 let signatory = output
//                     .address()
//                     .as_shelley()
//                     .and_then(|addr| addr.payment())
//                     .and_then(|payment_credential| payment_credential.as_key())
//                     .map(|key_hash| VerificationKey::from(&key_hash))
//                     .expect("Expected payment key credential for input which has no redeemer and extra signatory");
//                 self.signatories.extend(signatory.into_iter());
//                 self.extra_signatories.remove(&signatory);
//                 InputOutput::InputOnly(input_ref.clone(), None)
//             }
//         }
//         self.inputs_outputs.push(input_only)
//     }
// }
//
// pub mk_tx(
//     funding_utxos: &BTreeMap<Input, Output>,
//     script_utxo: &Utxo,
//     tx_steps: &TxSteps,
//     change_address: Address<address::kind::Any>,
//     protocol_parameters: &ProtocolParameters,
// ) -> anyhow::Result<Transaction<ReadyForSigning>> {
//     // Let's add all the utxos involved
//     // OLD CODE:
//     // let utxos = {
//     //     let mut base = funding_utxos.clone();
//     //     for ((input, output), _) in tx_steps.iter() {
//     //         base.insert(input.clone(), output.clone());
//     //     }
//     //     // add script utxo
//     //     base.insert(script_utxo.0.clone(), script_utxo.1.clone());
//     //     base
//     // };
//
//     // *

// pub fn batch(
//     network_id: &NetworkId,
//     protocol_parameters: &ProtocolParameters,
//     available_fuel: Utxos,
//     script_utxo: &(Input, Output),
//     channels: &Utxos,
//     intents: BTreeMap<Constants, Intent>,
//     opens: Vec<(Option<Credential>, Amount, Constants, Amount)>,
//schannel_sub_     change_address: Address<address::kind::Any>,
// ) -> Result<Transaction<ReadyForSigning>> {
//     let script_hash = Hash::from(script_utxo.1.script().ok_or(anyhow!("expect script"))?);
//     let all_channels = channels
//         .iter()
//         .map(|(i, o)| {
//             let res = match Channel::try_from_output(script_hash, o.clone()) {
//                 Err(err) => Err(anyhow!("Not a channel")),
//                 Ok(channel) => match intents.get(&channel.constants) {
//                     Some(intent) => Ok(CanStep::from_channel_intent(
//                         channel.clone(),
//                         intent.clone(),
//                     )),
//                     None => Err(anyhow!("No intent found. This could be fine")),
//                 },
//             };
//             (i.clone(), res)
//         })
//         .collect::<Vec<(Input, Result<CanStep>)>>();
//
//     let (good_inputs, good_channels) = all_channels
//         .into_iter()
//         .filter_map(|(i, res)| match res {
//             Ok(can_step) => match can_step {
//                 CanStep::Yes(_, _) => Some((i.clone(), can_step.clone())),
//                 _ => None,
//             },
//             _ => None,
//         })
//         .collect::<(Vec<Input>, Vec<CanStep>)>();
//
//     let steps = good_channels
//         .iter()
//         .filter_map(|cs| cs.as_step())
//         .collect::<Vec<Step>>();
//
//     let main_redeemer = Redeemer::new_main(Steps(steps));
//     let mut inputs: Vec<(Input, Option<PlutusData<'static>>)> = good_inputs
//         .iter()
//         .map(|i| (i.clone(), Some(PlutusData::from(Redeemer::Batch))))
//         .collect();
//
//     // Set main redeemer
//     if let Some(main_input) = inputs.first_mut() {
//         main_input.1 = Some(PlutusData::from(main_redeemer))
//     } else {
//         Err(anyhow!("No good inputs"))?;
//     }
//
//     // Add all the fuel
//     let mut fuel_inputs = available_fuel
//         .iter()
//         .map(|(i, _)| (i.clone(), None))
//         .collect();
//     inputs.append(&mut fuel_inputs);
//
//     let mut outputs = good_channels
//         .iter()
//         .filter_map(|cs| {
//             cs.as_channel()
//                 .map(|channel| channel.to_output(network_id.clone(), script_hash))
//         })
//         .collect::<Vec<Output>>();
//
//     let mut open_outputs = opens
//         .into_iter()
//         .map(|(delegation, amount, constants, subbed)| {
//             Channel::new(delegation, amount, constants, Stage::Opened(subbed))
//                 .to_output(network_id.clone(), script_hash)
//         })
//         .collect::<Vec<Output>>();
//
//     outputs.append(&mut open_outputs);
//
//     let constraints = good_channels
//         .iter()
//         .fold(Constraints::default(), |acc, curr| {
//             match curr.as_constraints() {
//                 Some(c) => acc.merge(c),
//                 None => acc,
//             }
//         });
//
//     // Gather all utxos
//     let mut utxos = channels.clone();
//     utxos.append(&mut available_fuel.clone());
//     utxos.insert(script_utxo.0.clone(), script_utxo.1.clone());
//
//     // FIXME :: These need to be added to the tx.
//     let lower_bound = constraints.lower_bound;
//     let upper_bound = constraints.upper_bound;
//     let specified_signatories: Vec<Hash<28>> = constraints
//         .required_signers
//         .iter()
//         .map(|x| <Hash<28>>::from(x.hash()))
//         .collect();
//
//     Transaction::build(&protocol_parameters, &utxos, |tx| {
//         tx.with_inputs(inputs.to_owned())
//             .with_outputs(outputs.to_owned())
//             .with_reference_inputs(vec![script_utxo.0.clone()])
//             .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
//             .with_specified_signatories(specified_signatories.to_owned())
//             .ok()
//     })
// }
