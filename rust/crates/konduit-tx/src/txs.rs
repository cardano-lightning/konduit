use anyhow::{Result, anyhow};
use konduit_data::{base::Amount, constants::Constants, step::Step};
use std::collections::BTreeMap;

use cardano_tx_builder::{
    Address, ChangeStrategy, Credential, Hash, Input, NetworkId, Output, PlutusData, PlutusScript,
    ProtocolParameters, Transaction, address, transaction::state::ReadyForSigning,
};

use crate::{Utxos, can_step::CanStep, channel::Channel, constraints::Constraints, intent::Intent};

pub fn deploy(
    protocol_parameters: &ProtocolParameters,
    utxos: &Utxos,
    script: PlutusScript,
    host_address: Address<address::kind::Any>,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>> {
    let inputs = vec![];
    let outputs = vec![Output::to(host_address).with_plutus_script(script)];
    Transaction::build(&protocol_parameters, &utxos, |tx| {
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
    opens: Vec<(Option<Credential>, Amount, Constants, Amount)>,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>> {
    let script_hash = Hash::from(script_utxo.1.script().ok_or(anyhow!("expect script"))?);
    let all_channels = channels
        .iter()
        .map(|(i, o)| {
            let res = match Channel::try_from_output(script_hash, o.clone()) {
                Err(err) => Err(anyhow!("Not a channel")),
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
        .collect::<Vec<(Input, Result<CanStep>)>>();

    let good_channels = all_channels
        .into_iter()
        .filter_map(|(i, ro)| match ro {
            Ok(active) => Some((i.clone(), active.clone())),
            _ => None,
        })
        .collect::<Vec<(Input, CanStep)>>();

    let inputs = good_channels
        .iter()
        .filter_map(|(i, cs)| {
            cs.as_step()
                .map(|step| (i.clone(), Some(PlutusData::from(step))))
        })
        .collect::<Vec<(Input, Option<PlutusData>)>>();

    let outputs = good_channels
        .iter()
        .filter_map(|(_, cs)| {
            cs.as_channel()
                .map(|channel| channel.to_output(network_id.clone(), script_hash))
        })
        .collect::<Vec<Output>>();

    let constraints =
        good_channels
            .iter()
            .fold(Constraints::default(), |acc, (_, curr)| {
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
    let lower_bound = constraints.lower_bound;
    let upper_bound = constraints.upper_bound;
    let specified_signatories: Vec<Hash<28>> = constraints
        .required_signers
        .iter()
        .map(|x| <Hash<28>>::from(x.hash()))
        .collect();

    Transaction::build(&protocol_parameters, &utxos, |tx| {
        tx.with_inputs(inputs.to_owned())
            .with_outputs(outputs.to_owned())
            .with_reference_inputs(vec![script_utxo.0.clone()])
            .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
            .with_specified_signatories(specified_signatories.to_owned())
            .ok()
    })
}
