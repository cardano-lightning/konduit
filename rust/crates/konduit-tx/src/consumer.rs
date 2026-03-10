use crate::{Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, SteppedUtxos, Utxos};
use anyhow::anyhow;
use cardano_sdk::{
    Address, Hash, Input, Output, Transaction, VerificationKey, address::kind,
    transaction::state::ReadyForSigning,
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
        .map(|o| crate::Open::new(o.amount, o.constant(wallet.clone()), None))
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

// fn mk_step(bounds: &Bounds, intents: &BTreeMap<Tag, Intent>, c: &ChannelOutput) -> Option<StepTo> {
//     match &c.stage {
//         Stage::Opened(subbed, useds) => {
//             match intents.get(&c.constants.tag)? {
//                 Intent::Add(add) => Some(StepTo::Cont(
//                     Cont::Add,
//                     Box::new(ChannelOutput {
//                         amount: add + c.amount,
//                         constants: c.constants.clone(),
//                         stage: Stage::Opened(*subbed, useds.clone()),
//                     }),
//                 )),
//                 Intent::Close => {
//                     // FIXME :: This coersion should not be necessary. Upstream a fix
//                     let elapse_at = Duration::from_millis(
//                         bounds.upper.as_millis() as u64
//                             + c.constants.close_period.as_millis() as u64,
//                     );
//                     Some(StepTo::Cont(
//                         Cont::Close,
//                         Box::new(ChannelOutput {
//                             amount: c.amount,
//                             constants: c.constants.clone(),
//                             stage: Stage::Closed(*subbed, useds.clone(), elapse_at),
//                         }),
//                     ))
//                 }
//             }
//         }
//         Stage::Closed(_, _, elapse_at) => {
//             if elapse_at.as_millis() < bounds.lower.as_millis() {
//                 Some(StepTo::Eol(Eol::Elapse))
//             } else {
//                 None
//             }
//         }
//         Stage::Responded(pendings_amount, pendings) => {
//             let unpends = pendings
//                 .iter()
//                 .map(|p| {
//                     if p.timeout.as_millis() < bounds.lower.as_millis() {
//                         Unpend::Expire
//                     } else {
//                         Unpend::Continue
//                     }
//                 })
//                 .collect::<Vec<_>>();
//             let claimable = c.amount - pendings_amount
//                 + pendings
//                     .iter()
//                     .zip(unpends.iter())
//                     .filter(|(_a, b)| matches!(b, Unpend::Expire))
//                     .map(|(a, _b)| a.amount)
//                     .sum::<u64>();
//             if claimable > 0 {
//                 let cont_pendings = pendings
//                     .iter()
//                     .zip(unpends.iter())
//                     .filter(|(_a, b)| matches!(b, Unpend::Continue))
//                     .map(|(a, _b)| a.clone())
//                     .collect::<Vec<_>>();
//                 if cont_pendings.is_empty() {
//                     Some(StepTo::Eol(Eol::End))
//                 } else {
//                     Some(StepTo::Cont(
//                         Cont::Expire(unpends),
//                         Box::new(ChannelOutput {
//                             amount: c.amount - claimable,
//                             constants: c.constants.clone(),
//                             stage: Stage::Responded(pendings_amount - claimable, cont_pendings),
//                         }),
//                     ))
//                 }
//             } else {
//                 None
//             }
//         }
//     }
// }

pub fn find_reference_script(utxos: &Utxos) -> Option<(Input, Output)> {
    utxos
        .iter()
        .find(|(_i, o)| {
            o.script()
                .is_some_and(|s| Hash::<28>::from(s) == KONDUIT_VALIDATOR.hash)
        })
        .map(|(i, o)| (i.clone(), o.clone()))
}
