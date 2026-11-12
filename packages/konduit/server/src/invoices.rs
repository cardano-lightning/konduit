//! invoices is a lookup table of `invoice`s

use konduit_core::{Duration, Lock, Secret};
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

/// Caller needs to insert verifying key to create an invoice response.
#[derive(Debug)]
pub struct Invoice {
    pub amount: u64,
    pub lock: Lock,
    pub timeout: Duration,
}

/// Storage-only: never leaves this module.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Entry {
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
pub struct Invoices {
    config: Config,
    invoices: Mutex<BTreeMap<Lock, Entry>>,
}

impl Invoices {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            invoices: Default::default(),
        }
    }

    fn invoices(&self) -> std::sync::MutexGuard<'_, BTreeMap<Lock, Entry>> {
        self.invoices.lock().expect("invoices state poisoned")
    }

    pub fn insert(&self, amount: u64, now: Duration) -> Invoice {
        let secret = Secret(rand::random::<[u8; 32]>());
        let lock = Lock::from(secret);
        let timeout = now + self.config.default_timeout;
        self.invoices()
            .insert(lock, Entry::new(amount, timeout, secret));
        Invoice {
            amount,
            lock,
            timeout,
        }
    }

    pub fn reveal(&self, lock: &Lock, amount: u64, now: Duration) -> Result<Secret, Error> {
        let entry = self.invoices().remove(lock).ok_or(Error::NotExist)?;
        if entry.timeout < now {
            return Err(Error::Expired);
        }
        if amount < entry.amount {
            return Err(Error::InsufficientAmount);
        }
        Ok(entry.secret)
    }

    pub fn secret(&self, lock: &Lock, now: Duration) -> Option<Secret> {
        self.invoices()
            .get(lock)
            .filter(|entry| entry.timeout >= now)
            .map(|entry| entry.secret)
    }

    /// Drops all entries past their timeout.
    pub fn prune(&self, now: Duration) {
        self.invoices().retain(|_, entry| entry.timeout >= now);
    }
}
