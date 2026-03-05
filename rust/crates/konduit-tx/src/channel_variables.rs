use std::{cmp, collections::BTreeMap};

use konduit_data::{Cont, Duration, Lock, Pending, Receipt, Secret, Stage, Unpend, Used};

use crate::step_and::{self, StepAnd};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Stage {0}, but Step is {1}")]
    Step(String, String),
    #[error("Other :: {0}")]
    Other(String),
}

/// Channel Variables aka Channel Data but without the constants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variables {
    amount: u64,
    stage: Stage,
}

impl Variables {
    pub fn new(amount: u64, stage: Stage) -> Self {
        Self { amount, stage }
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }
}

pub fn step(
    receipt: &Receipt,
    upper_bound: &Duration,
    variables: &Variables,
) -> Option<(Cont, Variables)> {
    match &variables.stage {
        Stage::Opened(subbed, useds) => {
            let squash = receipt.squash.clone();
            let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();
            // unexpired unused unlockeds
            let unlockeds = receipt
                .unlockeds()
                .iter()
                .filter(|c| !used_indexes.contains(&c.index()))
                .cloned()
                .collect::<Vec<_>>();
            // unsquashed useds
            let mut useds = useds
                .iter()
                .filter(|u| !squash.is_index_squashed(u.index))
                .cloned()
                .chain(unlockeds.iter().map(|u| Used::new(u.index(), u.amount())))
                .collect::<Vec<_>>();
            useds.sort_by_key(|i| i.index);
            let abs = squash.amount() + useds.iter().map(|u| u.amount).sum::<u64>();
            let rel = abs.checked_sub(*subbed)?;
            let actually_subable = cmp::min(rel, variables.amount);
            let subbed = subbed + actually_subable;
            let amount = variables.amount.saturating_sub(actually_subable);
            Some((
                Cont::Sub(receipt.squash.clone(), unlockeds),
                Variables {
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

            let actually_subable = cmp::min(rel, variables.amount);
            let amount = variables.amount.saturating_sub(actually_subable);
            Some((
                Cont::Respond(receipt.squash.clone(), cheques),
                Variables {
                    amount,
                    stage: Stage::Responded(pendings_amount, pendings),
                },
            ))
        }
        Stage::Responded(pendings_amount, pendings) => {
            let known = receipt
                .unlockeds()
                .iter()
                .map(|c| (c.lock().clone(), c.secret.clone()))
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
                Variables {
                    amount: variables.amount - claim,
                    stage: Stage::Responded(pendings_amount, pendings),
                },
            ))
        }
    }
}
