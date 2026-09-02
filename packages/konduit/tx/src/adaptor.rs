use std::collections::{BTreeMap, BTreeSet};

use cardano_sdk::{
    Address, Transaction, VerificationKey, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{AssetId, Duration};
use konduit_tmp::{Keytag, Receipt};

use crate::{
    ChannelUtxo, NetworkParameters, SteppedUtxos, Utxos, find_reference_script, to_verifying_key,
};

#[derive(Debug, Clone, thiserror::Error)]
#[error("insufficient total gain: preferences.min_total = {min_total}, gain = {gain}")]
pub struct InsufficientTotalGain {
    pub min_total: u64,
    pub gain: i128,
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
    upper: &Duration,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_utxo = find_reference_script(utxos);
    if reference_utxo.is_none() {
        return Err(anyhow::anyhow!("No konduit reference found"));
    };
    let change_address = wallet.to_address(network_parameters.network_id);
    let mut receipt_assets = BTreeMap::new();
    let mut skip = BTreeSet::new();
    for u in utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().sub_vkey == to_verifying_key(*wallet))
        .filter(|u| receipts.contains_key(&u.data().keytag()))
    {
        let keytag = u.data().keytag();
        let asset = u.data().constants().asset.clone();
        if receipt_assets
            .get(&keytag)
            .is_some_and(|expected| expected != &asset)
        {
            skip.insert(keytag);
            continue;
        }
        receipt_assets.insert(keytag, asset);
    }
    let groups = utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().sub_vkey == to_verifying_key(*wallet))
        .filter(|u| !skip.contains(&u.data().keytag()))
        .filter_map(|u| {
            let keytag = u.data().keytag();
            if receipt_assets.get(&keytag) != Some(&u.data().constants().asset) {
                return None;
            }
            receipts
                .get(&keytag)
                .and_then(|receipt| u.any_sub(receipt, upper).ok())
        })
        .filter(|u| u.gain_i128() >= i128::from(preferences.min_single))
        .fold(BTreeMap::<AssetId, Vec<_>>::new(), |mut groups, stepped| {
            groups
                .entry(stepped.data().channel().constants().asset.clone())
                .or_default()
                .push(stepped);
            groups
        });
    let gain = groups
        .values()
        .map(|group| group.iter().map(|step| step.gain_i128()).sum::<i128>())
        .max()
        .unwrap_or(0);
    let eligible = groups
        .into_values()
        .filter(|group| {
            group.iter().map(|step| step.gain_i128()).sum::<i128>()
                >= i128::from(preferences.min_total)
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err(InsufficientTotalGain {
            min_total: preferences.min_total,
            gain,
        }
        .into());
    }
    let steppeds = SteppedUtxos::from(eligible.into_iter().flatten().collect::<Vec<_>>());

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
