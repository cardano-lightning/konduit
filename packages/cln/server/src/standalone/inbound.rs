//! Inbound is one sender's inbound channel: their verifying key, tag,
//! backing amount, and our tracking receipt for what they've committed
//! to us. Verification of incoming `Locked` cheques and receipts
//! against key/tag happens here, not in the caller.
//!
//! No seeding: `receipt` starts `None` and stays `None` until something
//! else explicitly onboards this sender — every method here errors with
//! `NoReceipt` until that happens.

use crate::{Receipt, receipt, wire::sync::Receipt as WireReceipt};
use konduit_data::{Locked, Secret, SigningKey, Tag, Unverified, VerifyError, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const ADA: u64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub key: VerifyingKey,
    pub tag: Tag,
    pub backing: (u64, u64),
}

impl Default for Config {
    fn default() -> Self {
        let key = SigningKey::from([0; 32]).verifying_key();
        let tag = Tag::from(hex::decode("deadbeef").unwrap());
        Self {
            key,
            tag,
            backing: (100 * ADA, 0),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not verify locked cheque")]
    Unverified,
    #[error("no receipt yet")]
    NoReceipt,
    #[error("bad input")]
    Verify(#[from] VerifyError),
    #[error("receipt: {0}")]
    Receipt(#[from] receipt::Error),
}

pub struct Inbound {
    key: VerifyingKey,
    tag: Tag,
    #[allow(dead_code)]
    backing: (u64, u64),
    receipt: Option<Receipt>,
}

impl Inbound {
    pub fn new(config: Config) -> Self {
        Self {
            key: config.key,
            tag: config.tag,
            backing: config.backing,
            receipt: None,
        }
    }

    fn receipt_mut(&mut self) -> Result<&mut Receipt, Error> {
        self.receipt.as_mut().ok_or(Error::NoReceipt)
    }

    pub fn apply_locked(&mut self, locked: Locked<Unverified>) -> Result<(), Error> {
        // TODO :: add logic on backing
        let locked = locked
            .try_verify(&self.key, &self.tag)
            .map_err(|_| Error::Unverified)?;
        Ok(self.receipt_mut()?.apply_locked(locked)?)
    }

    pub fn apply_secret(&mut self, secret: Secret) -> Result<(), Error> {
        Ok(self.receipt_mut()?.apply_secret(secret)?)
    }

    pub fn apply_sync(&mut self, their: WireReceipt) -> Result<(), Error> {
        let theirs = Receipt::try_verify(their, &self.key, &self.tag)?;
        match &mut self.receipt {
            Some(ours) => Ok(ours.apply_sync(theirs)?),
            None => {
                self.receipt = Some(theirs);
                Ok(())
            }
        }
    }

    pub fn wire_receipt(&self) -> Result<WireReceipt, Error> {
        Ok(WireReceipt::from(
            self.receipt.as_ref().ok_or(Error::NoReceipt)?.clone(),
        ))
    }
}
