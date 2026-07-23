use std::collections::BTreeMap;

use async_trait::async_trait;
use konduit_data::{Keytag, Locked, Secret, Squash};

use crate::channel2;
use crate::{Channel, ChannelError, channel::Retainer};

#[async_trait]
pub trait Api: Send + Sync {
    // FIXME :: is this the right signature.
    // Assumption: There are distinct strategies:
    // - global failure ==> kill server,
    // - local failure ==> warn and continue
    async fn update_retainers(
        &self,
        retainers: BTreeMap<Keytag, Vec<Retainer>>,
    ) -> super::Result<BTreeMap<Keytag, Result<Channel, ChannelError>>>;

    async fn get_channel(&self, keytag: &Keytag) -> super::Result<Option<Channel>>;

    async fn get_all(&self) -> super::Result<BTreeMap<Keytag, Channel>>;

    async fn update_squash(&self, keytag: &Keytag, squash: Squash) -> super::Result<Channel>;

    async fn append_locked(&self, keytag: &Keytag, locked: Locked) -> super::Result<Channel>;

    async fn unlock(&self, keytag: &Keytag, secret: Secret) -> super::Result<Channel>;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Other")]
    Other,
}

pub trait Api2: Send + Sync {
    /// Fetch a channel by key.
    fn get(&self, key: &[u8]) -> Result<Option<Channel>, Error>;

    /// Insert a new channel. Errors if the key already exists.
    fn insert(&self, key: &[u8], channel: Channel) -> Result<(), Error>;

    /// Remove a channel by key. Errors if the key does not exist.
    fn remove(&self, key: &[u8]) -> Result<(), Error>;

    /// Iterate all channels.
    fn iter(&self) -> Result<Vec<(Vec<u8>, Channel)>, Error>;

    /// Modify an existing entry. Fails if absent.
    fn update<F, T>(&self, key: &[u8], f: F) -> Result<T, Error>
    where
        F: FnOnce(Channel) -> Result<(Channel, T), channel2::Error>;
}
