//! Transaction construction: fixed shapes on top of `Transaction::build`.
//! ASSUMPTIONs (unverified against the real crate): `Output::reference_script`,
//! `Output::with_reference_script`, `PlutusScript::hash`.

use cardano_sdk::{
    Address, ChangeStrategy, Hash, Input, Output, PlutusScript, ProtocolParameters, Transaction,
    Value, address, transaction::state::ReadyForSigning,
};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("building transaction: {0}")]
    Build(#[from] anyhow::Error),
    #[error("no utxo found holding the requested reference script")]
    ScriptNotFound,
}

pub const ONE_ADA_LOVELACE: u64 = 1_000_000;

pub fn protocol_parameters(network: &str) -> Option<ProtocolParameters> {
    match network {
        "mainnet" => Some(ProtocolParameters::mainnet()),
        "preprod" => Some(ProtocolParameters::preprod()),
        "preview" => Some(ProtocolParameters::preview()),
        _ => None,
    }
}

fn build(
    protocol_parameters: &ProtocolParameters,
    sender_utxos: &BTreeMap<Input, Output>,
    outputs: Vec<Output>,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>, Error> {
    let inputs = sender_utxos.keys().map(|input| (input.clone(), None));
    Ok(Transaction::build(
        protocol_parameters,
        sender_utxos,
        |tx| {
            tx.with_inputs(inputs.to_owned())
                .with_outputs(outputs.to_owned())
                .with_change_strategy(ChangeStrategy::as_last_output(change_address.to_owned()))
                .ok()
        },
    )?)
}

fn has_reference_script(output: &Output) -> bool {
    output.script().is_some()
}

fn scriptless(utxos: &BTreeMap<Input, Output>) -> BTreeMap<Input, Output> {
    utxos
        .iter()
        .filter(|(_, o)| !has_reference_script(o))
        .map(|(i, o)| (i.clone(), o.clone()))
        .collect()
}

/// Pay `receivers`, change to `change_address`. Skips script-bearing utxos.
pub fn send(
    protocol_parameters: &ProtocolParameters,
    sender_utxos: &BTreeMap<Input, Output>,
    receivers: Vec<(Address<address::kind::Any>, Value<u64>)>,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>, Error> {
    let outputs = receivers
        .into_iter()
        .map(|(a, v)| Output::new(a, v))
        .collect();
    build(
        protocol_parameters,
        &scriptless(sender_utxos),
        outputs,
        change_address,
    )
}

/// Send `lovelace` from `address` back to itself.
pub fn self_payment(
    protocol_parameters: &ProtocolParameters,
    utxos: &BTreeMap<Input, Output>,
    address: Address<address::kind::Any>,
    lovelace: u64,
) -> Result<Transaction<ReadyForSigning>, Error> {
    send(
        protocol_parameters,
        utxos,
        vec![(address.clone(), Value::new(lovelace))],
        address,
    )
}

/// Sweep everything, including script-bearing utxos, into one output at `to`.
pub fn empty(
    protocol_parameters: &ProtocolParameters,
    sender_utxos: &BTreeMap<Input, Output>,
    to: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>, Error> {
    build(protocol_parameters, sender_utxos, vec![], to)
}

pub fn upload(
    protocol_parameters: &ProtocolParameters,
    sender_utxos: &BTreeMap<Input, Output>,
    script: PlutusScript,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>, Error> {
    let output = Output::to(change_address.clone()).with_plutus_script(script);
    build(
        protocol_parameters,
        &scriptless(sender_utxos),
        vec![output],
        change_address,
    )
}

/// Reclaim the utxo(s) referencing `script_hash` to `change_address`.
pub fn teardown(
    protocol_parameters: &ProtocolParameters,
    sender_utxos: &BTreeMap<Input, Output>,
    script_hash: Hash<28>,
    change_address: Address<address::kind::Any>,
) -> Result<Transaction<ReadyForSigning>, Error> {
    let targeted: BTreeMap<Input, Output> = sender_utxos
        .iter()
        .filter(|(_, o)| o.script().is_some_and(|s| Hash::from(s) == script_hash))
        .map(|(i, o)| (i.clone(), o.clone()))
        .collect();
    if targeted.is_empty() {
        return Err(Error::ScriptNotFound);
    }
    build(protocol_parameters, &targeted, vec![], change_address)
}
