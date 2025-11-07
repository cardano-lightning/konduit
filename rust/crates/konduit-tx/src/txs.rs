use std::{cmp::min, collections::BTreeMap};

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

    let inputs = utxos.keys().map(|input| (input.clone(), None));
    Transaction::build(protocol_parameters, utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .ok()
    })
}

pub fn select_utxos(utxos: &Utxos, amount: Lovelace) -> anyhow::Result<Vec<&Input>> {
    if amount == 0 {
        return Ok(vec![]);
    }
    // Filter out utxos which hold reference scripts
    let mut sorted_utxos: Vec<(&Input, &Output)> = utxos
        .iter()
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

#[allow(clippy::too_many_arguments)]
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
    // TODO: We will reintroduce staking credential later on when it won't
    // cause UTxOs filtering issues.
    let contract_address = Address::from(
        Address::new(network_id, Credential::from_script(validator)),
        // .with_delegation(consumer_staking_credential.clone()),
    );
    let consumer_change_address = Address::from(
        Address::new(network_id, consumer_payment_credential),
        // .with_delegation(consumer_staking_credential),
    );

    let datum = PlutusData::from(Datum {
        own_hash: validator,
        constants: Constants {
            tag,
            add_vkey: consumer,
            sub_vkey: adaptor,
            close_period,
        },
        stage: Stage::Opened(0),
    });

    let funding_inputs: Vec<&Input> = select_utxos(utxos, amount + FEE_BUFFER)?;

    Transaction::build(protocol_parameters, utxos, |transaction| {
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

pub fn parse_output_datum(channel_output: &Output) -> anyhow::Result<konduit_data::Datum> {
    let datum_rc = channel_output
        .datum()
        .ok_or(anyhow!("Output has no datum"))?;
    match datum_rc {
        cardano::Datum::Inline(plutus_data) => Ok(konduit_data::Datum::from(plutus_data.clone())),
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
            min(available, owed - min(subbed_in, owed))
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
            Some((to_sub, channel_out))
        }
    } else {
        None
    }
}

pub const SUB_THRESHOLD: Lovelace = 0;

pub fn sub(
    // Utxos providing the necessary funds
    funding_utxos: &BTreeMap<Input, Output>,
    // Utxo holding the konduit script
    script_utxo: &Utxo,
    // Utxos representing the channels to be subtracted from
    // We assume that the channels where already filtered by tag and sub_vkey.
    channels_in: &[Utxo],
    // Receipt which authorizes the subtraction
    receipt: &konduit_data::Receipt,
    // Adaptor's verification key, allowed to *sub* funds
    // For now we use it as a baseline for the change and cash out address.
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
    #[allow(clippy::absurd_extreme_comparisons)]
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
        let selected_utxos = select_utxos(funding_utxos, FEE_BUFFER - min(to_sub, FEE_BUFFER))?;
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
        for (txout_ref, channel_out) in channels_in.iter() {
            base.insert(txout_ref.clone(), channel_out.clone());
        }
        base
    };

    let change_address = Address::from(
        Address::new(network_id, Credential::from_key(Hash::<28>::new(adaptor))), // TODO: this would break the Blockfrost UTxO selection which ignores staking part
                                                                                  // We chose for now to not use staking credentials at all.
                                                                                  // .with_delegation(Credential::from_key(Hash::<28>::new(adaptor.clone()))),
    );

    let specified_signatories = {
        let key_hash = Hash::<28>::new(adaptor);
        vec![key_hash]
    };
    let inputs = inputs
        .into_iter()
        .map(|(i, redeemer)| (i.clone(), redeemer))
        .collect::<Vec<(Input, Option<PlutusData>)>>();

    let collateral_inputs: Vec<Input> = {
        let selected = select_utxos(funding_utxos, FEE_BUFFER)?;
        selected.into_iter().cloned().collect::<Vec<Input>>()
    };

    let transaction = Transaction::build(protocol_parameters, &utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_collaterals(collateral_inputs.to_owned())
            .with_reference_inputs(vec![script_utxo.0.clone()])
            .with_specified_signatories(specified_signatories.to_owned())
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .ok()
    })?;
    Ok(Some(transaction))
}
