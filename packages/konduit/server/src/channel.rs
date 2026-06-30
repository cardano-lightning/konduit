//! A model of channel state according to the server.
//! This includes:
//! - Keytag ie the Channel id
//! - Bits of the L1 state
//! - The L2 state (ie receipt)
//! - Other account management such as last quote, and resource bucket.
//!
//! The DB can then be dumb ie agnostic to the domain.

use konduit_data::{Locked, Secret, Squash, Tag, VerifyingKey};
use konduit_wire::auth::squash::SquashProposal;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub mod receipt;
pub use receipt::Receipt;

mod error;
pub use error::Error;

mod bucket;
use bucket::Bucket;

mod backing;
use backing::Backing;

use crate::{
    channel::backing::Opened,
    time::{self, now},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Channel {
    /// Channel id
    #[n(0)]
    key: VerifyingKey,
    /// Channel id
    #[n(1)]
    tag: Tag,
    /// L1 state. Cached for serving `./state.
    /// Use external service prior to quote.
    /// FIXME :: Does this even make sense?
    #[n(2)]
    backing: Backing,
    /// L2 state
    #[n(3)]
    receipt: Option<Receipt>,
    /// Resourcing
    #[n(4)]
    bucket: Bucket,
    /// Pending state such as last quote.
    #[n(5)]
    cache: Cache,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    bucket_capacity: u64,
    #[n(1)]
    bucket_refill_rate: u64,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode, Default)]
pub struct Cache {
    #[n(0)]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    quote: Option<Vec<u8>>,
}

impl Channel {
    /// TODO :: The bucket is not plumbed in.
    /// Any read or write to the channel should consume from the bucket.
    /// The specific amounts need to be configured.
    pub fn new(config: &Config, key: VerifyingKey, tag: Tag) -> Self {
        Self {
            key,
            tag,
            backing: Backing::default(),
            receipt: None,
            bucket: Bucket::new(
                config.bucket_capacity,
                config.bucket_refill_rate,
                time::now(),
            ),
            cache: Cache::default(),
        }
    }

    // --- Accessors ----------------------------------------------------------

    pub fn key(&self) -> &VerifyingKey {
        &self.key
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn receipt(&self) -> Option<&Receipt> {
        self.receipt.as_ref()
    }

    pub fn backing(&self) -> &Backing {
        &self.backing
    }

    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }

    fn bucket_mut(&mut self) -> &mut Bucket {
        &mut self.bucket
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    fn receipt_mut(&mut self) -> Result<&mut Receipt, Error> {
        self.receipt.as_mut().ok_or(Error::NoReceipt)
    }

    // --- Events -------------------------------------------------------------

    /// Apply a consumer-signed squash.
    /// Creates the receipt if this is the first squash; advances it otherwise.
    pub fn apply_squash(&mut self, squash: Squash) -> Result<(), Error> {
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
    /// Verifies the signature and checks available funds before accepting.
    pub fn apply_locked(&mut self, locked: Locked) -> Result<(), Error> {
        let locked = locked.try_verify(&self.key, &self.tag)?;
        let available = self.spendable()?;
        if locked.amount() > available {
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

    /// Apply backing. Replace what is there
    pub fn apply_backing(&mut self, backing: Backing) {
        self.backing = backing;
    }

    /// Apply Opened.
    pub fn apply_opened(&mut self, amount: u64, subbed: u64, created_at: u64) {
        self.backing.push(amount, subbed, created_at);
    }

    /// Apply closed. Effectivly drop backing.
    pub fn apply_closed(&mut self) {
        self.backing = Backing::default()
    }

    /// Apply quote caches the quote in the cache
    /// Note the type erasure
    pub fn apply_quote(&mut self, quote: Vec<u8>) {
        self.cache.quote = Some(quote);
    }

    /// Apply commit: gives quote to caller and clears cache.
    pub fn apply_commit(&mut self) -> Option<Vec<u8>> {
        self.cache.quote.take()
    }

    // --- Queries ------------------------------------------------------------

    /// The next squash the server expects the client to sign.
    pub fn propose_squash(&self) -> Result<SquashProposal, Error> {
        let proposal = self.receipt().ok_or(Error::NoReceipt)?.propose_squash()?;
        Ok(proposal)
    }

    /// Amount available for new payments, given current on-chain backing and receipt state.
    /// FIXME :: this should be an external
    pub fn spendable(&self) -> Result<u64, Error> {
        todo!()
    }

    /// The index to assign to the next cheque.
    pub fn next_index(&self) -> Result<u64, Error> {
        todo!()
    }
}

// --- Ops ------------------------------------------------------------

pub fn consume(amount: u64) -> impl FnOnce(Channel) -> Result<(Channel, Option<()>), Error> {
    move |mut channel| {
        channel.bucket_mut().consume(amount, now())?;
        Ok((channel, None))
    }
}
