use cardano_sdk::VerificationKey;
use konduit_data::{
    Keytag, L1Channel, Locked, Receipt, Secret, Squash, SquashProposal, Stage, Step, Tag, Used,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::cmp;

/// A channel is an edge in the Lightning network.
/// In Konduit, a channel is from Consumer to Adaptor.
///
/// What a `channel` actually is, is a subtle business.
/// The design of Konduit does not enforce a uniqueness of well-formed UTXOs
/// at tip at the Konduit script address.
/// Konduit design does require some chain liveness to guarantee safety.
/// More precisely, if Adaptor sees tip (confirmed tip) `< close_period / 2`,
/// then they are safe upto general chain safety (eg tx failures due to chain congestion).
/// also does not depend on "tracing" state through channels.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aux {
    is_active: bool,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Retainer required")]
    NoRetainer,
    #[error("Receipt required")]
    NoReceipt,
    #[error("Not Active")]
    NotActive,
    #[error("Input not well-formed")]
    BadInput,
    #[error("Insufficient capacity")]
    Capacity,
    #[error("Insufficient funds")]
    Funds,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Channel2 {
    #[serde_as(as = "serde_with::hex::Hex")]
    key: VerificationKey,
    #[serde_as(as = "serde_with::hex::Hex")]
    tag: Tag,
    retainer: Option<Retainer>,
    receipt: Option<Receipt>,
    aux: Aux,
}

impl Channel2 {
    pub fn new(keytag: Keytag) -> Self {
        let (key, tag) = keytag.split();
        Channel2 {
            key,
            tag,
            retainer: None,
            receipt: None,
            aux: Aux { is_active: true },
        }
    }

    pub fn assert_active(&self) -> Result<(), Error> {
        if !self.aux.is_active {
            return Err(Error::NotActive);
        }
        Ok(())
    }

    pub fn receipt(&self) -> Option<Receipt> {
        self.receipt.clone()
    }

    pub fn squash_proposal(&self) -> Result<SquashProposal, Error> {
        match &self.receipt {
            None => Err(Error::NoReceipt),
            Some(receipt) => receipt.squash_proposal().map_err(|_e| Error::BadInput),
        }
    }

    /// How much funds are currently unspent, uncommitted.
    /// Error if no funds can be spent because of other reasons.
    pub fn potentially_subable(&self) -> Result<u64, Error> {
        self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(Error::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(Error::NoReceipt)?;
        if receipt.capacity() == 0 {
            return Err(Error::Capacity);
        };
        let abs = receipt.potentially_subable(&retainer.useds);
        let rel = abs.saturating_sub(retainer.subbed);
        Ok(retainer.amount.saturating_sub(rel))
    }

    pub fn next_index(&self) -> Result<u64, Error> {
        self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(Error::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(Error::NoReceipt)?;
        let index = match retainer.useds.last() {
            None => receipt.max_index(),
            Some(u) => cmp::max(receipt.max_index(), u.index),
        };
        Ok(index + 1)
    }

    // ---- Modifiers ----

    pub fn update_retainer(&mut self, l1s: Vec<Retainer>) -> Result<(), Error> {
        // FIXME :: Handle Useds better!
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

    pub fn update_squash(&mut self, squash: Squash) -> Result<bool, Error> {
        let _ = self.assert_active();
        if !squash.verify(&self.key, &self.tag) {
            return Err(Error::BadInput);
        };
        match &mut self.receipt {
            None => {
                self.receipt = Some(Receipt::new(squash));
                Ok(true)
            }
            Some(receipt) => Ok(receipt.update_squash(squash)),
        }
    }

    pub fn append_locked(&mut self, locked: Locked) -> Result<(), Error> {
        if !locked.verify(&self.key, &self.tag) {
            Err(Error::BadInput)
        } else if locked.amount() > self.potentially_subable()? {
            Err(Error::Funds)
        } else {
            self.receipt
                .as_mut()
                .expect("Impossible")
                .append_locked(locked)
                .map_err(|_err| Error::BadInput)
        }
    }

    pub fn unlock(&mut self, secret: Secret) -> Result<(), Error> {
        self.receipt
            .as_mut()
            .ok_or(Error::NoReceipt)?
            .unlock(secret)
            .map_err(|_err| Error::BadInput)
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

impl TryFrom<&L1Channel> for Retainer {
    type Error = anyhow::Error;

    fn try_from(value: &L1Channel) -> Result<Self, Self::Error> {
        let Stage::Opened(subbed, useds) = value.stage.clone() else {
            return Err(anyhow::anyhow!("Not openened"));
        };
        let amount = value.amount;
        Ok(Retainer {
            amount,
            subbed,
            useds,
        })
    }
}

impl TryFrom<&konduit_tx::Channel> for Retainer {
    type Error = anyhow::Error;

    fn try_from(value: &konduit_tx::Channel) -> Result<Self, Self::Error> {
        let Stage::Opened(subbed, useds) = value.stage().clone() else {
            return Err(anyhow::anyhow!("Not openened"));
        };
        let amount = value.amount();
        Ok(Retainer {
            amount,
            subbed,
            useds,
        })
    }
}
