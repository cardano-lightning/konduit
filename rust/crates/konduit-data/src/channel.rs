use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::cmp;

use cardano_tx_builder::VerificationKey;

use crate::{Keytag, L1Channel, Locked, Receipt, Secret, Squash, SquashProposal, Step, Tag, Used};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aux {
    is_active: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChannelError {
    #[error("Retainer required")]
    NoRetainer,
    #[error("Receipt required")]
    NoReceipt,
    #[error("Not Active")]
    NotActive,
    #[error("Input not well-formed")]
    BadInput,
    #[error("Not enough funds")]
    Amount,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel {
    #[serde_as(as = "serde_with::hex::Hex")]
    key: VerificationKey,
    #[serde_as(as = "serde_with::hex::Hex")]
    tag: Tag,
    retainer: Option<Retainer>,
    receipt: Option<Receipt>,
    aux: Aux,
}

impl Channel {
    pub fn new(keytag: Keytag) -> Self {
        let (key, tag) = keytag.split();
        Channel {
            key,
            tag,
            retainer: None,
            receipt: None,
            aux: Aux { is_active: true },
        }
    }

    pub fn assert_active(&self) -> Result<(), ChannelError> {
        if self.aux.is_active == false {
            return Err(ChannelError::NotActive);
        }
        Ok(())
    }

    pub fn update_retainer(&mut self, l1s: Vec<Retainer>) -> Result<(), ChannelError> {
        self.retainer = match &self.receipt {
            None => l1s.into_iter().max_by_key(|l1| l1.amount),
            Some(receipt) => l1s.into_iter().max_by_key(|l1| {
                (
                    cmp::min(
                        receipt
                            .potentially_subable(&l1.useds)
                            .saturating_sub(l1.subbed),
                        l1.amount,
                    ),
                    l1.amount,
                )
            }),
        };
        Ok(())
    }

    pub fn update_squash(&mut self, squash: Squash) -> Result<bool, ChannelError> {
        let _ = self.assert_active();
        if !squash.verify(&self.key, &self.tag) {
            return Err(ChannelError::BadInput);
        };
        match &mut self.receipt {
            None => {
                self.receipt = Some(Receipt::new(squash));
                Ok(true)
            }
            Some(receipt) => Ok(receipt.update_squash(squash)),
        }
    }

    pub fn squash_proposal(&self) -> Result<SquashProposal, ChannelError> {
        match &self.receipt {
            None => Err(ChannelError::NoReceipt),
            Some(receipt) => receipt
                .squash_proposal()
                .map_err(|_e| ChannelError::BadInput),
        }
    }

    pub fn capacity(&self) -> Result<u64, ChannelError> {
        // FIXME :: NEED A MAX UNSQUASHED VERIFICATION STEP
        let _ = self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(ChannelError::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(ChannelError::NoReceipt)?;
        let abs = receipt.potentially_subable(&retainer.useds);
        let rel = abs.saturating_sub(retainer.subbed);
        Ok(retainer.amount.saturating_sub(rel))
    }

    pub fn next_index(&self) -> Result<u64, ChannelError> {
        let _ = self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(ChannelError::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(ChannelError::NoReceipt)?;
        let index = match retainer.useds.last() {
            None => receipt.max_index(),
            Some(u) => cmp::max(receipt.max_index(), u.index),
        };
        Ok(index + 1)
    }

    pub fn append_locked(&mut self, locked: Locked) -> Result<(), ChannelError> {
        if !locked.verify(&self.key, &self.tag) {
            return Err(ChannelError::BadInput);
        } else if locked.amount() > self.capacity()? {
            return Err(ChannelError::Amount);
        } else {
            self.receipt
                .as_mut()
                .expect("Impossible")
                .append_locked(locked)
                .map_err(|_err| ChannelError::BadInput)
        }
    }

    pub fn unlock(&mut self, secret: Secret) -> Result<(), ChannelError> {
        self.receipt
            .as_mut()
            .ok_or(ChannelError::NoReceipt)?
            .unlock(secret)
            .map_err(|_err| ChannelError::BadInput)
    }

    /// We need to verify that if the channel is active, then there is
    /// still a potential retainer with at as much capacity.
    /// FIXME :: There is still a potential issue with useds here.
    pub fn steps(
        &self,
        _l1_channels: &Vec<L1Channel>,
    ) -> Result<Vec<Option<(Step, L1Channel)>>, ChannelError> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    index: u64,
    amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retainer {
    amount: u64,
    subbed: u64,
    useds: Vec<Used>,
}
