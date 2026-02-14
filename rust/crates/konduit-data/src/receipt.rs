use std::{cmp, collections::BTreeMap};

use anyhow::anyhow;

use cardano_tx_builder::VerificationKey;
use serde::{Deserialize, Serialize};

use crate::{
    Cheque, ChequeBody, Cont, Duration, Indexes, L1Channel, Lock, Locked, MAX_UNSQUASHED, Pending,
    Secret, Squash, SquashBody, SquashBodyError, SquashProposal, Stage, Tag, Unlocked, Unpend,
    Used,
};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ReceiptError {
    #[error("Squash cannot include a (locked) cheque.")]
    IncludesCheque,

    #[error("Squash body error {0}")]
    SquashBody(SquashBodyError),

    #[error("Squash body was not reproduced")]
    NotReproduced,

    #[error("Bad input")]
    BadInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub squash: Squash,
    pub cheques: Vec<Cheque>,
}

impl Receipt {
    pub fn new(squash: Squash) -> Self {
        Self {
            squash,
            cheques: vec![],
        }
    }

    pub fn new_with_cheques(squash: Squash, cheques: Vec<Cheque>) -> anyhow::Result<Self> {
        if cheques.len() > MAX_UNSQUASHED {
            Err(anyhow!("Too many unsquashed"))?;
        }
        let mut sorted: Vec<Cheque> = vec![];
        for cheque in cheques {
            let index = cheque.index();
            if squash.body.is_index_squashed(index) {
                Err(anyhow!("Index {} is already squashed", index))?;
            }
            match sorted.binary_search_by(|probe| probe.index().cmp(&index)) {
                Ok(_) => Err(anyhow!("Duplicate index {}", index))?,
                Err(position) => sorted.insert(position, cheque),
            };
        }
        sorted.sort();
        Ok(Self::new_no_verify(squash, sorted))
    }

    pub fn new_no_verify(squash: Squash, cheques: Vec<Cheque>) -> Self {
        Self { squash, cheques }
    }

    pub fn max_index(&self) -> u64 {
        match self.cheques.last() {
            Some(mc) => cmp::max(self.squash.index(), mc.index()),
            None => self.squash.index(),
        }
    }

    pub fn verify_components(&self, key: &VerificationKey, tag: &Tag) -> bool {
        self.squash.verify(key, tag)
            && self.cheques.iter().all(|m| m.verify(key, tag))
            && match Self::new_with_cheques(self.squash.clone(), self.cheques.clone()) {
                Ok(copy) => self == &copy,
                Err(_) => false,
            }
    }

    // FIXME :: Not currently used
    pub fn capacity(&self) -> usize {
        MAX_UNSQUASHED - self.cheques.len()
    }

    pub fn useds(&self, useds: Vec<Used>) -> Vec<Used> {
        useds
            .into_iter()
            .filter(|used| !self.squash.body.is_index_squashed(used.index))
            .chain(self.unlockeds().iter().map(|unlocked| {
                let ChequeBody { index, amount, .. } = unlocked.body;
                Used::new(index, amount)
            }))
            .collect::<Vec<_>>()
    }

    pub fn lockeds(&self) -> Vec<Locked> {
        self.cheques
            .iter()
            .filter_map(|x| x.as_locked())
            .collect::<Vec<Locked>>()
    }

    pub fn unlockeds(&self) -> Vec<Unlocked> {
        self.cheques
            .iter()
            .filter_map(|x| x.as_unlocked())
            .collect::<Vec<Unlocked>>()
    }

    pub fn unlock(&mut self, secret: Secret) -> Result<(), String> {
        let lock = Lock::from(secret.clone());
        let mut none_changed = true;
        self.cheques.iter_mut().for_each(|i| {
            if let Cheque::Locked(locked) = i
                && lock == locked.body.lock
            {
                *i = Cheque::Unlocked(Unlocked::new(locked.clone(), secret.clone()).unwrap());
                none_changed = true;
            }
        });
        match none_changed {
            true => Ok(()),
            false => Err("None changed".to_string()),
        }
    }

    pub fn owed(&self) -> u64 {
        self.squash.amount() + self.unlockeds().iter().map(|x| x.body.amount).sum::<u64>()
    }

    pub fn committed(&self) -> u64 {
        self.squash.amount() + self.cheques.iter().map(|x| x.amount()).sum::<u64>()
    }

    pub fn potentially_subable(&self, useds: &[Used]) -> u64 {
        let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();
        let unused: u64 = self
            .cheques
            .iter()
            .filter(|c| !used_indexes.contains(&c.index()))
            .map(|c| c.amount())
            .sum();
        let unsquashed: u64 = useds
            .iter()
            .filter(|u| !self.squash.is_index_squashed(u.index))
            .map(|u| u.amount)
            .sum();
        self.squash.amount() + unused + unsquashed
    }

    pub fn currently_subable(&self, useds: &[Used]) -> u64 {
        let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();
        let unused: u64 = self
            .unlockeds()
            .iter()
            .filter(|c| !used_indexes.contains(&c.index()))
            .map(|c| c.amount())
            .sum();
        let unsquashed: u64 = useds
            .iter()
            .filter(|u| !self.squash.is_index_squashed(u.index))
            .map(|u| u.amount)
            .sum();
        self.squash.amount() + unused + unsquashed
    }

    /// Time and signature must already be verified
    pub fn insert(&mut self, locked: Locked) -> anyhow::Result<()> {
        let index = locked.body.index;
        let cheque = Cheque::from(locked);
        match self
            .cheques
            .binary_search_by(|probe| probe.index().cmp(&index))
        {
            Ok(_) => Err(anyhow!("Duplicate index {}", &index))?,
            Err(position) => self.cheques.insert(position, cheque),
        };
        Ok(())
    }

    pub fn append_locked(&mut self, locked: Locked) -> Result<(), ReceiptError> {
        if locked.index() > self.max_index() {
            self.cheques.push(Cheque::from(locked));
            Ok(())
        } else {
            Err(ReceiptError::BadInput)
        }
    }

    /// Drop all locked cheques for which timeout is < now
    pub fn timeout(&mut self, now: Duration) {
        self.cheques.retain(|c| {
            let Some(l) = c.as_locked() else {
                return true;
            };
            l.timeout() > now
        })
    }

    pub fn update_squash(&mut self, squash: Squash) -> bool {
        let squashed: u64 = self
            .cheques
            .iter()
            .filter(|c| squash.is_index_squashed(c.index()))
            .map(|c| c.amount())
            .sum();
        if squash.amount() >= self.squash.amount() + squashed {
            self.cheques
                .retain(|c| !squash.is_index_squashed(c.index()));
            self.squash = squash;
            true
        } else {
            false
        }
    }

    pub fn squash_proposal(&self) -> Result<SquashProposal, ReceiptError> {
        let current = self.squash.clone();
        let unlockeds = self.unlockeds();
        let index = match unlockeds.last() {
            Some(u) => cmp::max(self.squash.index(), u.index()),
            None => self.squash.index(),
        };
        let exclude = Indexes::new(
            self.lockeds()
                .iter()
                .map(|l| l.index())
                .filter(|i| i < &index)
                .collect(),
        )
        .map_err(|_e| ReceiptError::NotReproduced)?;
        let proposal = SquashBody::new(self.owed(), index, exclude)
            .map_err(|_e| ReceiptError::NotReproduced)?;
        Ok(SquashProposal {
            proposal,
            current,
            unlockeds,
        })
    }

    pub fn step(&self, upper_bound: &Duration, l1: &L1Channel) -> Option<(Cont, L1Channel)> {
        match &l1.stage {
            Stage::Opened(subbed, useds) => {
                let squash = self.squash.clone();
                let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();
                let unlockeds = self
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
                    Cont::Sub(self.squash.clone(), unlockeds),
                    L1Channel {
                        amount,
                        stage: Stage::Opened(subbed, useds),
                    },
                ))
            }
            Stage::Closed(subbed, useds, _) => {
                let squash = self.squash.clone();
                let used_indexes: Vec<u64> = useds.iter().map(|u| u.index).collect();

                let cheques = self
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
                    Cont::Respond(self.squash.clone(), cheques),
                    L1Channel {
                        amount,
                        stage: Stage::Responded(pendings_amount, pendings),
                    },
                ))
            }
            Stage::Responded(pendings_amount, pendings) => {
                let known = self
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
                    L1Channel {
                        amount: l1.amount - claim,
                        stage: Stage::Responded(pendings_amount, pendings),
                    },
                ))
            }
        }
    }
}
