use crate::{Utxos, can_step::CanStep, channel::Channel, constraints::Constraints, intent::Intent};
use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{
    Address, ChangeStrategy, Credential, Hash, Input, NetworkId, Output, PlutusData, PlutusScript,
    ProtocolParameters, Transaction, Value, VerificationKey, address,
    transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Datum, Redeemer, Stage, Step, Steps, Tag, TimeDelta};
use std::collections::BTreeMap;

pub async fn open(
    // A backend to Cardano
    connector: impl CardanoConnect,
    // Konduit's validator hash,
    validator: Hash<28>,
    // Quantity of Lovelace to deposit into the channel
    amount: u64,
    // Consumer's verification key, allowed to *add* funds.
    consumer: VerificationKey,
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: VerificationKey,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: Tag,
    // Minimum time from `close` to `elapse`.
    close_period: TimeDelta,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let consumer_payment_credential = Credential::from_key(Hash::<28>::new(consumer));

    let resolved_inputs = connector
        .utxos_at(&consumer_payment_credential, None)
        .await?;

    let from: Input = resolved_inputs
        .iter()
        .find(|(_, output)| output.value().lovelace() > amount)
        .map(|(input, _)| input.clone())
        .ok_or(anyhow!(
            "no sufficiently large output found in consumer's wallet"
        ))?;

    let network_id = NetworkId::from(connector.network());

    let contract_address =
        Address::from(Address::new(network_id, Credential::from_script(validator)));

    let consumer_change_address =
        Address::from(Address::new(network_id, consumer_payment_credential));

    let datum = PlutusData::from(Datum::new(
        validator,
        Constants {
            tag,
            add_vkey: consumer,
            sub_vkey: adaptor,
            close_period: close_period.0.as_millis() as u64,
        },
        Stage::Opened(amount),
    ));

    Transaction::build(
        &connector.protocol_parameters().await?,
        &resolved_inputs,
        |transaction| {
            transaction
                .with_inputs([(from.clone(), None)])
                .with_outputs([Output::new(contract_address.clone(), Value::new(amount))
                    .with_datum(datum.clone())])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    consumer_change_address.clone(),
                ))
                .ok()
        },
    )
}

pub fn deploy(
    protocol_parameters: &ProtocolParameters,
    utxos: &Utxos,
    script: PlutusScript,
    host_address: Address<address::kind::Any>,
    change_address: Address<address::kind::Any>,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let inputs = vec![];
    let outputs = vec![Output::to(host_address).with_plutus_script(script)];
    Transaction::build(protocol_parameters, utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .ok()
    })
}

pub fn batch(
    network_id: &NetworkId,
    protocol_parameters: &ProtocolParameters,
    available_fuel: Utxos,
    script_utxo: &(Input, Output),
    channels: &Utxos,
    intents: BTreeMap<Constants, Intent>,
    opens: Vec<(Option<Credential>, u64, Constants, u64)>,
    change_address: Address<address::kind::Any>,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let script_hash = Hash::from(script_utxo.1.script().ok_or(anyhow!("expect script"))?);
    let all_channels = channels
        .iter()
        .map(|(i, o)| {
            let res = match Channel::try_from_output(script_hash, o.clone()) {
                Err(e) => Err(anyhow!(e).context("not a channel")),
                Ok(channel) => match intents.get(&channel.constants) {
                    Some(intent) => Ok(CanStep::from_channel_intent(
                        channel.clone(),
                        intent.clone(),
                    )),
                    None => Err(anyhow!("No intent found. This could be fine")),
                },
            };
            (i.clone(), res)
        })
        .collect::<Vec<(Input, anyhow::Result<CanStep>)>>();

    let (good_inputs, good_channels) = all_channels
        .into_iter()
        .filter_map(|(i, res)| match res {
            Ok(can_step) => match can_step {
                CanStep::Yes(_, _) => Some((i.clone(), can_step.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect::<(Vec<Input>, Vec<CanStep>)>();

    let steps = good_channels
        .iter()
        .filter_map(|cs| cs.as_step())
        .collect::<Vec<Step>>();

    let main_redeemer = Redeemer::new_main(Steps(steps));
    let mut inputs: Vec<(Input, Option<PlutusData<'static>>)> = good_inputs
        .iter()
        .map(|i| (i.clone(), Some(PlutusData::from(Redeemer::Batch))))
        .collect();

    // Set main redeemer
    if let Some(main_input) = inputs.first_mut() {
        main_input.1 = Some(PlutusData::from(main_redeemer))
    } else {
        Err(anyhow!("No good inputs"))?;
    }

    // Add all the fuel
    let mut fuel_inputs = available_fuel
        .iter()
        .map(|(i, _)| (i.clone(), None))
        .collect();
    inputs.append(&mut fuel_inputs);

    let mut outputs = good_channels
        .iter()
        .filter_map(|cs| {
            cs.as_channel()
                .map(|channel| channel.to_output(*network_id, script_hash))
        })
        .collect::<Vec<Output>>();

    let mut open_outputs = opens
        .into_iter()
        .map(|(delegation, amount, constants, subbed)| {
            Channel::new(delegation, amount, constants, Stage::Opened(subbed))
                .to_output(*network_id, script_hash)
        })
        .collect::<Vec<Output>>();

    outputs.append(&mut open_outputs);

    let constraints = good_channels
        .iter()
        .fold(Constraints::default(), |acc, curr| {
            match curr.as_constraints() {
                Some(c) => acc.merge(c),
                None => acc,
            }
        });

    // Gather all utxos
    let mut utxos = channels.clone();
    utxos.append(&mut available_fuel.clone());
    utxos.insert(script_utxo.0.clone(), script_utxo.1.clone());

    // FIXME :: These need to be added to the tx.
    let _lower_bound = constraints.lower_bound;
    let _upper_bound = constraints.upper_bound;
    let specified_signatories: Vec<Hash<28>> = constraints
        .required_signers
        .iter()
        .map(Hash::<28>::new)
        .collect();

    Transaction::build(protocol_parameters, &utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_reference_inputs(vec![script_utxo.0.clone()])
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .with_specified_signatories(specified_signatories.to_owned())
            .ok()
    })
}
