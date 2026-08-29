//! A model of channel state according to the server.
//! This includes:
//! - Keytag ie the Channel id
//! - Bits of the L1 state
//! - The L2 state (ie receipt)
//! - Other account management such as last quote, and resource bucket.
//!
//! The DB can then be dumb ie agnostic to the domain.

use std::cmp;

use cardano_sdk::VerificationKey;
use konduit_data::{Locked, Secret, Squash, Stage, Tag, Unverified, Used, VerifyingKey};
use konduit_tmp::{Keytag, Receipt, SquashProposal, from_verifying_key, receipt, to_verifying_key};

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::VerifyError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("channel not active")]
    NotActive,
    #[error("no retainer: channel not funded on-chain")]
    NoRetainer,
    #[error("no receipt: submit a null squash first")]
    NoReceipt,
    #[error("insufficient capacity: too many unresolved payments")]
    Capacity,
    #[error("insufficient funds")]
    Funds,
    // #[error("limit: {0}")]
    // Limit(#[from] bucket::Error),
    #[error("bad input")]
    Input,
    #[error("verify failed")]
    Verify,
    #[error("receipt: {0}")]
    Receipt(#[from] receipt::Error),
}

impl From<VerifyError> for Error {
    fn from(_value: VerifyError) -> Self {
        Self::Verify
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Aux {
    #[n(0)]
    is_active: bool,
}

impl Default for Aux {
    fn default() -> Self {
        Self { is_active: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Channel {
    /// Channel id
    #[n(0)]
    key: VerifyingKey,
    /// Channel id
    #[n(1)]
    tag: Tag,
    /// L1 state. Cached for serving `./auth/state`.
    /// Use external service prior to quote.
    /// FIXME :: Does this even make sense?
    #[n(2)]
    retainer: Option<Retainer>,
    /// L2 state
    #[n(3)]
    receipt: Option<Receipt>,
    #[n(4)]
    aux: Aux,
    // /// Resourcing
    // #[n(5)]
    // bucket: Bucket,
    // /// Pending state such as last quote.
    // #[n(6)]
    // cache: Cache,
}

// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
// pub struct Config {
//     #[n(0)]
//     bucket_capacity: u64,
//     #[n(1)]
//     bucket_refill_rate: u64,
// }

// #[serde_as]
// #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
// pub struct Cache {
//     #[n(0)]
//     #[serde_as(as = "Option<serde_with::hex::Hex>")]
//     #[serde(skip_serializing_if = "Option::is_none")]
//     quote: Option<Vec<u8>>,
// }

impl Channel {
    /// TODO :: The bucket is not plumbed in.
    /// Any read or write to the channel should consume from the bucket.
    /// The specific amounts need to be configured.
    pub fn new(
        //config: &Config,
        key: VerifyingKey,
        tag: Tag,
    ) -> Self {
        Self {
            key,
            tag,
            retainer: None,
            receipt: None,
            aux: Aux { is_active: true },
            // bucket: Bucket::new(
            //     config.bucket_capacity,
            //     config.bucket_refill_rate,
            //     time::now(),
            // ),
            // cache: Cache::default(),
        }
    }

    pub fn new_with(
        keytag: &Keytag,
        retainer: Option<Retainer>,
        receipt: Option<Receipt>,
        aux: Aux,
    ) -> Self {
        let (key, tag) = keytag.split();
        Self {
            key: to_verifying_key(key),
            tag,
            retainer,
            receipt,
            aux,
        }
    }

    // --- Accessors ----------------------------------------------------------

    pub fn key(&self) -> &VerifyingKey {
        &self.key
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn receipt(&self) -> &Option<Receipt> {
        &self.receipt
    }

    pub fn retainer(&self) -> &Option<Retainer> {
        &self.retainer
    }

    pub fn aux(&self) -> &Aux {
        &self.aux
    }

    // pub fn bucket(&self) -> &Bucket {
    //     &self.bucket
    // }

    // fn bucket_mut(&mut self) -> &mut Bucket {
    //     &mut self.bucket
    // }

    // pub fn cache(&self) -> &Cache {
    //     &self.cache
    // }

    pub fn keytag(&self) -> Keytag {
        Keytag::new(&self.verification_key(), self.tag())
    }

    pub fn verification_key(&self) -> VerificationKey {
        from_verifying_key(self.key)
    }

    pub fn assert_active(&self) -> Result<(), Error> {
        if !self.aux.is_active {
            return Err(Error::NotActive);
        }
        Ok(())
    }
    // --- Queries ------------------------------------------------------------

    /// How much funds are currently uncommitted (available to be committed).
    /// Error if no funds can be spent because of other reasons.
    /// Assumes retainer is in a state of prev squash ie nothing weird happened.
    pub fn uncommitted(&self) -> Result<u64, Error> {
        self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(Error::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(Error::NoReceipt)?;
        if receipt.capacity() == 0 {
            return Err(Error::Capacity);
        };
        let abs_committed = receipt.committed();
        let rel_committed = abs_committed.saturating_sub(retainer.subbed);
        Ok(retainer.amount.saturating_sub(rel_committed))
    }

    /// Error if cannot commit.
    pub fn can_commit(&self, x: u64) -> Result<u64, Error> {
        if self.uncommitted()? > x {
            self.next_index()
        } else {
            Err(Error::Funds)
        }
    }

    pub fn next_index(&self) -> Result<u64, Error> {
        self.assert_active()?;
        let retainer = self.retainer.as_ref().ok_or(Error::NoRetainer)?;
        let receipt = self.receipt.as_ref().ok_or(Error::NoReceipt)?;
        Ok(cmp::max(
            retainer.useds.last().map_or(0, |u| u.index),
            receipt.propose_index(),
        ))
    }

    /// The next squash the server expects the client to sign.
    pub fn propose_squash(&self) -> Result<SquashProposal, Error> {
        let proposal = self
            .receipt()
            .as_ref()
            .ok_or(Error::NoReceipt)?
            .propose_squash()?;
        Ok(proposal)
    }

    // --- Modifiers --------------------------------------------------------

    fn receipt_mut(&mut self) -> Result<&mut Receipt, Error> {
        self.receipt.as_mut().ok_or(Error::NoReceipt)
    }

    // --- Events -------------------------------------------------------------

    pub fn apply_retainer(&mut self, candidates: Vec<Retainer>) -> Result<(), Error> {
        // FIXME :: Handle Useds better!Currently assumes nothing weird happened.
        self.retainer = match &self.receipt {
            None => candidates.into_iter().max_by_key(|l1| l1.amount),
            Some(receipt) => candidates.into_iter().max_by_key(|l1| {
                (
                    cmp::min(
                        // FIXME :: This is now incorrect, but will leave to the indexer proxy
                        receipt.committed().saturating_sub(l1.subbed),
                        l1.amount,
                    ),
                    l1.amount,
                )
            }),
        };
        Ok(())
    }

    /// Apply a consumer-signed squash.
    /// Creates the receipt if this is the first squash; advances it otherwise.
    /// Will error if the squash is not later than current
    pub fn apply_squash(&mut self, squash: Squash<Unverified>) -> Result<(), Error> {
        let squash = squash.try_verify(&self.key, &self.tag)?;
        match &mut self.receipt {
            None => {
                self.receipt = Some(Receipt::new(squash));
            }
            Some(r) => {
                r.apply_squash(squash)?;
            }
        }
        Ok(())
    }

    /// Append a consumer-signed locked cheque.
    /// Verifies the signature and checks uncommitted funds before accepting.
    pub fn apply_locked(&mut self, locked: Locked) -> Result<(), Error> {
        let locked = locked.try_verify(&self.key, &self.tag)?;
        if locked.amount() > self.uncommitted()? {
            return Err(Error::Funds);
        }
        self.receipt_mut()?.apply_locked(locked)?;
        Ok(())
    }

    /// Resolve a locked cheque with its payment preimage aka secret.
    pub fn apply_secret(&mut self, secret: Secret) -> Result<(), Error> {
        self.receipt_mut()?.apply_secret(secret)?;
        Ok(())
    }

    /// Resolve multiple secrets
    pub fn apply_secrets(&mut self, secrets: Vec<Secret>) -> Result<(), Error> {
        for secret in secrets.into_iter() {
            self.receipt_mut()?.apply_secret(secret)?;
        }
        Ok(())
    }

    // /// Apply quote caches the quote in the cache
    // /// Note the type erasure
    // /// NOT YET USED
    // pub fn apply_quote(&mut self, quote: Vec<u8>) {
    //     self.cache.quote = Some(quote);
    // }

    // /// Apply commit: gives quote to caller and clears cache.
    // /// NOT YET USED
    // pub fn apply_commit(&mut self) -> Option<Vec<u8>> {
    //     self.cache.quote.take()
    // }
}

// --- Ops ------------------------------------------------------------

// /// NOT USED UNTIL BUCKETS ARE FULLY IMPLEMENTED
//
// pub fn consume(amount: u64) -> impl FnOnce(Channel2) -> Result<(Channel2, Option<()>), Error> {
//     move |mut channel| {
//         channel.bucket_mut().consume(amount, now())?;
//         Ok((channel, None))
//     }
// }

pub fn apply_locked(locked: Locked) -> impl FnOnce(Channel) -> Result<Channel, Error> {
    move |mut channel| {
        channel.apply_locked(locked)?;
        Ok(channel)
    }
}

pub fn apply_squash(squash: Squash) -> impl FnOnce(Channel) -> Result<Channel, Error> {
    move |mut channel| {
        channel.apply_squash(squash)?;
        Ok(channel)
    }
}

pub fn apply_secrets(secrets: Vec<Secret>) -> impl FnOnce(Channel) -> Result<Channel, Error> {
    move |mut channel| {
        channel.apply_secrets(secrets)?;
        Ok(channel)
    }
}

pub fn upsert_retainers(
    retainers: Vec<Retainer>,
) -> impl FnOnce(Channel) -> Result<Channel, Error> {
    move |mut channel| {
        channel.apply_retainer(retainers)?;
        Ok(channel)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Retainer {
    #[n(0)]
    pub amount: u64,
    #[n(1)]
    pub subbed: u64,
    #[n(2)]
    pub useds: Vec<Used>,
}

// impl TryFrom<&L1Channel> for Retainer {
//     type Error = anyhow::Error;
//
//     fn try_from(value: &L1Channel) -> Result<Self, Self::Error> {
//         let Stage::Opened(subbed, useds) = value.stage.clone() else {
//             return Err(anyhow::anyhow!("Not openened"));
//         };
//         let amount = value.amount;
//         Ok(Retainer {
//             amount,
//             subbed,
//             useds,
//         })
//     }
// }

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
