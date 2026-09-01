//! paymes is a lookup table of `payme`s

use crate::{random, time::now};
use konduit_data::{Duration, Lock, Secret};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    pub default_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(300),
        }
    }
}

/// Caller needs to insert verifying key to create a payme response.
#[derive(Debug)]
pub(crate) struct Payme {
    pub(crate) amount: u64,
    pub(crate) lock: Lock,
    pub(crate) timeout: Duration,
}

/// Storage-only: never leaves this module.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub(crate) struct Entry {
    #[n(0)]
    amount: u64,
    #[n(1)]
    timeout: Duration,
    #[n(2)]
    secret: Secret,
}

impl Entry {
    fn new(amount: u64, timeout: Duration, secret: Secret) -> Self {
        Self {
            amount,
            timeout,
            secret,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("not exist")]
    NotExist,
    #[error("expired")]
    Expired,
    #[error("insufficient amount")]
    InsufficientAmount,
}

#[derive(Debug)]
pub struct Paymes {
    config: Config,
    paymes: Mutex<BTreeMap<Lock, Entry>>,
}

impl Paymes {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            paymes: Default::default(),
        }
    }

    fn paymes(&self) -> std::sync::MutexGuard<'_, BTreeMap<Lock, Entry>> {
        self.paymes.lock().expect("paymes state poisoned")
    }

    pub(crate) fn insert(&self, amount: u64) -> Payme {
        let secret = Secret(random::arr32());
        let lock = Lock::from(secret);
        let timeout = now() + self.config.default_timeout;
        self.paymes()
            .insert(lock.clone(), Entry::new(amount, timeout, secret));
        Payme {
            amount,
            lock,
            timeout,
        }
    }

    pub(crate) fn reveal(&self, lock: &Lock, amount: u64) -> Result<Secret, Error> {
        let entry = self.paymes().remove(lock).ok_or(Error::NotExist)?;
        if entry.timeout < now() {
            return Err(Error::Expired);
        }
        if amount < entry.amount {
            return Err(Error::InsufficientAmount);
        }
        Ok(entry.secret)
    }

    /// Drops all entries past their timeout.
    pub fn prune(&self) {
        let now = now();
        self.paymes().retain(|_, entry| entry.timeout >= now);
    }
}
