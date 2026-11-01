//! The outbound lookup: which peer this rail can reach, and how.
//!
//! Keyed by the peer's verifying key, which for a one-hop rail is also
//! the payee an instruction names. Entries are seeded from config and settable
//! while running, so a new outbound peer does not mean a restart.
//!
//! Onward routing — a payee that is not the peer — would need a route
//! table rather than a peer table, and this is not one.

use crate::{Channel, Receipt, channel};
use konduit_data::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

/// The seed for the lookup: one entry per peer, each naming that peer's
/// key and url.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub channels: Vec<channel::Config>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            channels: vec![Default::default()],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a peer: {0:?}")]
    NoRoute(VerifyingKey),
}

#[derive(Default)]
pub struct Channels {
    channels: RwLock<BTreeMap<VerifyingKey, Arc<Channel>>>,
}

impl Channels {
    pub fn new() -> Self {
        Self::default()
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, BTreeMap<VerifyingKey, Arc<Channel>>> {
        self.channels.read().expect("channels state poisoned")
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, BTreeMap<VerifyingKey, Arc<Channel>>> {
        self.channels.write().expect("channels state poisoned")
    }

    /// Adds or replaces the channel to `key`. Replacing drops whatever
    /// receipt state the old entry held, so this is for a peer whose url
    /// changed or a peer newly configured — not a way to reset a live
    /// channel.
    pub fn upsert(&self, key: VerifyingKey, channel: Channel) -> Option<Arc<Channel>> {
        self.write().insert(key, Arc::new(channel))
    }

    pub fn remove(&self, key: &VerifyingKey) -> Option<Arc<Channel>> {
        self.write().remove(key)
    }

    pub fn get(&self, peer: &VerifyingKey) -> Result<Arc<Channel>, Error> {
        self.read().get(peer).cloned().ok_or(Error::NoRoute(*peer))
    }

    pub fn contains(&self, peer: &VerifyingKey) -> bool {
        self.read().contains_key(peer)
    }

    /// A snapshot, so a caller can iterate without holding the lock.
    pub fn all(&self) -> Vec<(VerifyingKey, Arc<Channel>)> {
        self.read().iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    /// The peers this rail can pay, as an admin query answers it.
    pub fn peers(&self) -> Vec<VerifyingKey> {
        self.read().keys().copied().collect()
    }
}

impl Channels {
    /// Builds the lookup from config. `receipt` is called per entry
    /// because each channel starts from its own signed null squash.
    pub fn from_config(
        config: &Config,
        mut receipt: impl FnMut(&channel::Config) -> Receipt,
        mut auth: impl FnMut(&channel::Config) -> String,
    ) -> Self {
        let channels = Self::new();
        for entry in config.channels.iter() {
            channels.upsert(entry.key, Channel::new(entry, receipt(entry), auth(entry)));
        }
        channels
    }
}
