use std::collections::BTreeMap;

use cardano_sdk::{
    Address, PosixMilliseconds, Transaction, VerificationKey, address::kind,
    transaction::state::ReadyForSigning,
};
use konduit_core::{Keytag, Receipt, Stage};

use crate::{
    ChannelUtxo, NetworkParameters, SteppedUtxos, Utxos, find_reference_script, to_verifying_key,
};

#[derive(Debug, Clone, thiserror::Error)]
#[error("insufficient total gain: preferences.min_total = {min_total}, gain = {gain}")]
pub struct InsufficientTotalGain {
    pub min_total: u64,
    pub gain: i64,
}

#[derive(Debug, Clone)]
pub struct AdaptorPreferences {
    // Prevents spending a utxo that would result in too little gain relative to the cost of inclusion.
    pub min_single: u64,
    // Prevents a transaction in which the total gain is too little
    pub min_total: u64,
}

// WARNING :: This transaction does **not** verify that the resultant tx does not
// violate the condition that if the channel is being treated as active,
// then the retainer is not responded.
// This must be handled elsewhere!
pub fn tx(
    network_parameters: &NetworkParameters,
    preferences: &AdaptorPreferences,
    wallet: &VerificationKey,
    receipts: &BTreeMap<Keytag, Receipt>,
    utxos: &Utxos,
    upper: &PosixMilliseconds,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_utxo = find_reference_script(utxos);
    if reference_utxo.is_none() {
        return Err(anyhow::anyhow!("No konduit reference found"));
    };
    let change_address = wallet.to_address(network_parameters.network_id);
    let steppeds = utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().sub_vkey == to_verifying_key(*wallet))
        .filter_map(|u| {
            // Closed means this step is a Respond, not a Sub: the owner can
            // elapse the channel (taking everything) the moment the close
            // period is up, so this is the counterparty's only window to
            // claim what it's owed and must always go through, however
            // small. Decide that before `any_sub` consumes `u`.
            let is_respond = matches!(u.data().stage(), Stage::Closed(..));
            let receipt = receipts.get(&u.data().keytag())?;
            let stepped = u.any_sub(receipt, upper).ok()?;
            Some((is_respond, stepped))
        })
        .filter(|(is_respond, stepped)| {
            *is_respond || stepped.gain() >= preferences.min_single as i64
        })
        .collect::<Vec<_>>();
    let has_respond = steppeds.iter().any(|(is_respond, _)| *is_respond);
    let steppeds = SteppedUtxos::from(
        steppeds
            .into_iter()
            .map(|(_, stepped)| stepped)
            .collect::<Vec<_>>(),
    );

    // Sub is opportunistic and skipped below min_total to avoid paying fees
    // for a negligible claim, but a pending Respond is mandatory, so it
    // must never be blocked by that same fee-efficiency gate.
    if !has_respond && steppeds.gain() < preferences.min_total as i64 {
        return Err(InsufficientTotalGain {
            min_total: preferences.min_total,
            gain: steppeds.gain(),
        }
        .into());
    }

    let opens = vec![];

    let wallet_address: Address<kind::Any> =
        wallet.to_address(network_parameters.network_id).into();

    let fuel = utxos
        .iter()
        .filter(|u| u.1.address() == &wallet_address)
        .map(|u| (u.0.clone(), u.1.clone()))
        .to_owned()
        .collect::<BTreeMap<_, _>>();
    crate::tx::tx(
        network_parameters,
        reference_utxo.as_ref(),
        change_address.into(),
        steppeds,
        opens,
        &fuel,
    )
}
