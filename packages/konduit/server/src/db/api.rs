use std::collections::BTreeMap;

use async_trait::async_trait;
use konduit_data::{Locked, Secret, Squash};
use konduit_tmp::Keytag;

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

    /// FIXME :: should be verified
    async fn update_squash(&self, keytag: &Keytag, squash: Squash) -> super::Result<Channel>;

    /// FIXME :: should be verified
    async fn append_locked(&self, keytag: &Keytag, locked: Locked) -> super::Result<Channel>;

    /// FIXME :: should be verified
    async fn unlock(&self, keytag: &Keytag, secret: Secret) -> super::Result<Channel>;
}
