//! Inbounds is the table of per-sender Inbound channels, keyed by
use super::inbound::{self, Inbound};
use crate::wire::{auth::Keytag, sync::Receipt as WireReceipt};
use konduit_data::{Locked, Secret};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub inbounds: Vec<inbound::Config>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            inbounds: vec![Default::default()],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown inbound channel")]
    NotFound,
    #[error("inbound: {0}")]
    Inbound(#[from] inbound::Error),
}

pub struct Inbounds {
    inbounds: Mutex<BTreeMap<Keytag, Inbound>>,
}

impl Inbounds {
    pub fn new(config: Config) -> Self {
        let channels = config
            .inbounds
            .into_iter()
            .map(|c| {
                let keytag = Keytag::from((&c.key, &c.tag));
                (keytag, Inbound::new(c))
            })
            .collect();
        Self {
            inbounds: Mutex::new(channels),
        }
    }

    fn inbounds(&self) -> std::sync::MutexGuard<'_, BTreeMap<Keytag, Inbound>> {
        self.inbounds.lock().expect("inbounds state poisoned")
    }

    pub fn contains(&self, keytag: &Keytag) -> Result<(), Error> {
        if self.inbounds().contains_key(keytag) {
            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    pub fn apply_locked(&self, keytag: &Keytag, locked: Locked) -> Result<(), Error> {
        Ok(self
            .inbounds()
            .get_mut(keytag)
            .ok_or(Error::NotFound)?
            .apply_locked(locked)?)
    }

    pub fn apply_secret(&self, keytag: &Keytag, secret: Secret) -> Result<(), Error> {
        Ok(self
            .inbounds()
            .get_mut(keytag)
            .ok_or(Error::NotFound)?
            .apply_secret(secret)?)
    }

    pub fn apply_sync(&self, keytag: &Keytag, their: WireReceipt) -> Result<(), Error> {
        Ok(self
            .inbounds()
            .get_mut(keytag)
            .ok_or(Error::NotFound)?
            .apply_sync(their)?)
    }

    pub fn wire_receipt(&self, keytag: &Keytag) -> Result<WireReceipt, Error> {
        Ok(self
            .inbounds()
            .get(keytag)
            .ok_or(Error::NotFound)?
            .wire_receipt()?)
    }
}
