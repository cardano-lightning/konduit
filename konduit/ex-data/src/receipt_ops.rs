use crate::{L1Channel, SquashProposal};
use konduit_data::{
    Cont, Duration, Indexes, Lock, Pending, Receipt, ReceiptError, Secret, Stage, Unpend, Used,
};
use std::{cmp, collections::BTreeMap};

pub fn squash_proposal(receipt: &Receipt) -> Result<SquashProposal, ReceiptError> {
    let current = receipt.squash.clone();
    let unlockeds = receipt.unlockeds();
    let index = match unlockeds.last() {
        Some(u) => cmp::max(receipt.squash.index(), u.index()),
        None => receipt.squash.index(),
    };
    let exclude = Indexes::new(
        receipt
            .lockeds()
            .iter()
            .map(|l| l.index())
            .filter(|i| i < &index)
            .collect(),
    )
    .map_err(|_e| ReceiptError::NotReproduced)?;
    let proposal = konduit_data::SquashBody::new(receipt.owed(), index, exclude)
        .map_err(|_e| ReceiptError::NotReproduced)?;
    Ok(SquashProposal {
        proposal,
        current,
        unlockeds,
        lockeds: receipt.lockeds(),
    })
}

pub fn step(
    receipt: &Receipt,
    upper_bound: &Duration,
    l1: &L1Channel,
) -> Option<(Cont, L1Channel)> {
    match &l1.stage {
        Stage::Opened(subbed, useds) => {
            let squash = receipt.squash.clone();
            let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();
            let unlockeds = receipt
                .unlockeds()
                .iter()
                .filter(|c| !used_indexes.contains(&c.index()))
                .cloned()
                .collect::<Vec<_>>();
            let mut useds = useds
                .iter()
                .filter(|u| !squash.is_index_squashed(u.index))
                .cloned()
                .chain(unlockeds.iter().map(|u| Used::new(u.index(), u.amount())))
                .collect::<Vec<_>>();
            useds.sort_by_key(|i| i.index);
            let abs = squash.amount() + useds.iter().map(|u| u.amount).sum::<u64>();
            let rel = abs.checked_sub(*subbed)?;
            let actually_subable = cmp::min(rel, l1.amount);
            let subbed = subbed + actually_subable;
            let amount = l1.amount.saturating_sub(actually_subable);
            Some((
                Cont::Sub(receipt.squash.clone(), unlockeds),
                L1Channel {
                    amount,
                    stage: Stage::Opened(subbed, useds),
                },
            ))
        }
        Stage::Closed(subbed, useds, _) => {
            let squash = receipt.squash.clone();
            let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();

            let cheques = receipt
                .cheques
                .iter()
                .filter(|c| !used_indexes.contains(&c.index()))
                .cloned()
                .collect::<Vec<_>>();

            let pendings = cheques
                .iter()
                .filter_map(|c| c.as_locked())
                .map(Pending::from)
                .collect::<Vec<_>>();
            let pendings_amount = pendings.iter().map(|p| p.amount).sum::<u64>();

            let unused = cheques
                .iter()
                .filter_map(|c| c.as_unlocked())
                .map(|c| c.amount())
                .sum::<u64>();

            let unsquashed = useds
                .iter()
                .filter(|u| !squash.is_index_squashed(u.index))
                .map(|u| u.amount)
                .sum::<u64>();

            let abs = squash.amount() + unsquashed + unused;

            if *subbed > abs && pendings_amount == 0 {
                return None;
            }

            let rel = abs.saturating_sub(*subbed);

            let actually_subable = cmp::min(rel, l1.amount);
            let amount = l1.amount.saturating_sub(actually_subable);
            Some((
                Cont::Respond(receipt.squash.clone(), cheques),
                L1Channel {
                    amount,
                    stage: Stage::Responded(pendings_amount, pendings),
                },
            ))
        }
        Stage::Responded(pendings_amount, pendings) => {
            let known = receipt
                .unlockeds()
                .iter()
                .map(|c| (*c.lock(), c.secret.clone()))
                .collect::<BTreeMap<Lock, Secret>>();
            let unpends: Vec<Unpend> = pendings
                .iter()
                .filter(|p| p.timeout > *upper_bound)
                .map(|u| known.get(&u.lock).map_or(Unpend::Continue, Unpend::from))
                .collect::<Vec<_>>();
            let claim = pendings
                .iter()
                .zip(&unpends)
                .filter(|(_a, b)| b.is_continue())
                .map(|(a, _b)| a.amount)
                .sum::<u64>();
            if claim == 0 {
                return None;
            }
            let pendings = pendings
                .iter()
                .zip(&unpends)
                .filter(|(_a, b)| !b.is_continue())
                .map(|(a, _b)| a.clone())
                .collect::<Vec<_>>();
            let pendings_amount = pendings_amount - claim;
            Some((
                Cont::Unlock(unpends),
                L1Channel {
                    amount: l1.amount - claim,
                    stage: Stage::Responded(pendings_amount, pendings),
                },
            ))
        }
    }
}
