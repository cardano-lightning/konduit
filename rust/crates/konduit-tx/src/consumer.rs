use crate::{Bounds, ChannelUtxo, NetworkParameters, SteppedUtxos, Utxos, find_reference_script};
use cardano_sdk::{
    Address, Transaction, VerificationKey, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Duration, Stage, Tag};
use std::collections::BTreeMap;

pub struct OpenIntent {
    pub tag: Tag,
    pub sub_vkey: VerificationKey,
    pub close_period: Duration,
    pub amount: u64,
}

impl OpenIntent {
    fn constant(self, add_vkey: VerificationKey) -> Constants {
        Constants {
            tag: self.tag,
            add_vkey,
            sub_vkey: self.sub_vkey,
            close_period: self.close_period,
        }
    }
}

pub enum Intent {
    Add(u64),
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
        .filter(|u| u.data().constants().add_vkey == *wallet);

    let steppeds = consumer_channels
        .filter_map(|u| match u.data().stage() {
            Stage::Opened(_, _) => match intents.get(&u.data().constants().tag)? {
                Intent::Add(amount) => u.add(*amount).ok(),
                Intent::Close => u
                    .close(&bounds.upper.expect("Must have upper bound for close"))
                    .ok(),
            },
            Stage::Closed(_, _, _) => bounds.lower.and_then(|lower| u.elapse(&lower).ok()),
            Stage::Responded(_, pendings) => {
                if pendings.is_empty() {
                    u.end(bounds.lower.as_ref()).ok()
                } else {
                    bounds.lower.and_then(|lower| u.expire(&lower).ok())
                }
            }
        })
        .collect::<Vec<_>>();
    let steppeds = SteppedUtxos::from(steppeds);

    let opens = opens
        .into_iter()
        .map(|o| crate::Open::new(o.amount, o.constant(*wallet), None))
        .collect::<Vec<_>>();

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
