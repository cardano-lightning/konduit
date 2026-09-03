use std::collections::BTreeMap;

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
    /// Ada-denominated minimums.
    pub min_single: u64,
    pub min_total: u64,
    /// Raw-unit minimums for explicitly enabled native assets.
    pub asset_minimums: BTreeMap<AssetId, (u64, u64)>,
}

impl AdaptorPreferences {
    fn minimums(&self, asset: &AssetId) -> Option<(u64, u64)> {
        match asset {
            AssetId::Ada => Some((self.min_single, self.min_total)),
            _ => self.asset_minimums.get(asset).copied(),
        }
    }
}

// WARNING :: This transaction does **not** verify that the resultant tx does not
// violate the condition that if the channel is being treated as active,
// then the retainer is not responded.
// This must be handled elsewhere!
pub fn tx(
    network_parameters: &NetworkParameters,
    preferences: &AdaptorPreferences,
    wallet: &VerificationKey,
    receipts: &BTreeMap<Keytag, (AssetId, Receipt)>,
    utxos: &Utxos,
    upper: &Duration,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_utxo = find_reference_script(utxos);
    if reference_utxo.is_none() {
        return Err(anyhow::anyhow!("No konduit reference found"));
    };
    let change_address = wallet.to_address(network_parameters.network_id);
    let groups = utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().sub_vkey == to_verifying_key(*wallet))
        .filter_map(|u| {
            let keytag = u.data().keytag();
            let (asset, receipt) = receipts.get(&keytag)?;
            if asset != &u.data().constants().asset {
                return None;
            }
            let (min_single, _) = preferences.minimums(asset)?;
            let stepped = u.any_sub(receipt, upper).ok()?;
            (stepped.gain_i128() >= i128::from(min_single)).then_some(stepped)
        })
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
        .into_iter()
        .filter_map(|(asset, group)| {
            let (_, min_total) = preferences.minimums(&asset)?;
            (group.iter().map(|step| step.gain_i128()).sum::<i128>() >= i128::from(min_total))
                .then_some(group)
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
