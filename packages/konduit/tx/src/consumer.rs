use crate::{
    Bounds, ChannelUtxo, NetworkParameters, SteppedUtxo, SteppedUtxos, Utxos,
    find_reference_script, to_verifying_key,
};
use cardano_sdk::{
    Address, Transaction, VerificationKey, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{AssetId, Constants, Duration, Stage, Tag};
use std::collections::{BTreeMap, BTreeSet};

pub struct OpenIntent {
    pub tag: Tag,
    pub sub_vkey: VerificationKey,
    pub close_period: Duration,
    pub amount: u64,
    pub asset: AssetId,
}

impl OpenIntent {
    fn constant(self, add_vkey: VerificationKey) -> Constants {
        Constants {
            tag: self.tag,
            add_vkey: to_verifying_key(add_vkey),
            sub_vkey: to_verifying_key(self.sub_vkey),
            close_period: self.close_period,
            asset: self.asset,
        }
    }
}

pub enum Intent {
    Add { amount: u64, asset: AssetId },
    Close,
}

pub fn tx(
    network_parameters: &NetworkParameters,
    wallet: &VerificationKey,
    opens: Vec<OpenIntent>,
    intents: BTreeMap<Tag, Intent>,
    utxos: &Utxos,
    bounds: Bounds,
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let reference_utxo = find_reference_script(utxos);

    let change_address = wallet.to_address(network_parameters.network_id);

    let consumer_channels = utxos
        .iter()
        .filter_map(|u| ChannelUtxo::try_from(u).ok())
        .filter(|u| u.data().constants().add_vkey == to_verifying_key(*wallet));

    let mut unmatched_adds = intents
        .iter()
        .filter_map(|(tag, intent)| match intent {
            Intent::Add { .. } => Some(tag.clone()),
            Intent::Close => None,
        })
        .collect::<BTreeSet<_>>();

    let mut steppeds = Vec::<SteppedUtxo>::new();
    for u in consumer_channels {
        let tag = u.data().constants().tag.clone();
        let channel_asset = u.data().constants().asset.clone();
        let stepped = match u.data().stage() {
            Stage::Opened(_, _) => match intents.get(&tag) {
                Some(Intent::Add { amount, asset }) if asset == &channel_asset => {
                    unmatched_adds.remove(&tag);
                    Some(u.add(*amount).map_err(|(_, error)| error)?)
                }
                Some(Intent::Add { .. }) => None,
                Some(Intent::Close) => Some(
                    u.close(&bounds.upper.expect("Must have upper bound for close"))
                        .map_err(|(_, error)| error)?,
                ),
                None => None,
            },
            Stage::Closed(_, _, _) => bounds.lower.and_then(|lower| u.elapse(&lower).ok()),
            Stage::Responded(_, pendings) => {
                if pendings.is_empty() {
                    u.end(bounds.lower.as_ref()).ok()
                } else {
                    bounds.lower.and_then(|lower| u.expire(&lower).ok())
                }
            }
        };
        if let Some(stepped) = stepped {
            steppeds.push(stepped);
        }
    }
    if !unmatched_adds.is_empty() {
        anyhow::bail!("add intent did not match a channel: {unmatched_adds:?}");
    }
    let steppeds = SteppedUtxos::from(steppeds);

    let opens = opens
        .into_iter()
        .map(|o| crate::Open::new(o.amount, o.constant(*wallet), None))
        .collect::<Vec<_>>();

    if opens.is_empty() && steppeds.inputs().is_empty() {
        return Err(anyhow::anyhow!("Empty tx"));
    }

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
