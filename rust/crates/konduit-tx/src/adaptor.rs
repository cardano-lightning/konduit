use std::collections::BTreeMap;

use cardano_sdk::{
    Address, Transaction, VerificationKey, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{Duration, Keytag, Receipt};

use crate::{ChannelUtxo, NetworkParameters, SteppedUtxos, Utxos, find_reference_script};

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
    upper: &Duration,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_utxo = find_reference_script(utxos);
    if reference_utxo.is_none() {
        return Err(anyhow::anyhow!("No konduit reference found"));
    };
    let change_address = wallet.to_address(network_parameters.network_id);
    let steppeds = utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().sub_vkey == *wallet)
        .filter_map(|u| {
            receipts
                .get(&u.data().keytag())
                .and_then(|receipt| u.any_sub(receipt, upper).ok())
        })
        .filter(|u| u.gain() >= preferences.min_single as i64)
        .collect::<Vec<_>>();
    let steppeds = SteppedUtxos::from(steppeds);

    if steppeds.gain() < preferences.min_total as i64 {
        return Err(anyhow::anyhow!(
            "insufficient total gain: preferences.min_total = {}, gain = {}",
            preferences.min_total,
            steppeds.gain()
        ));
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
