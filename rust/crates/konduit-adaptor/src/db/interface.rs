use std::collections::BTreeMap;

use async_trait::async_trait;
use konduit_data::{Keytag, Locked, Secret, Squash};

use crate::{Channel, channel::Retainer};

pub type DbResult<T> = Result<T, super::DbError>;

#[async_trait]
pub trait DbInterface: Send + Sync {
    // FIXME :: is this the right signature.
    // Assumption: There are distinct strategies:
    // - global failure ==> kill server,
    // - local failure ==> warn and continue
    async fn update_retainers(
        &self,
        retainers: BTreeMap<Keytag, Vec<Retainer>>,
    ) -> DbResult<BTreeMap<Keytag, DbResult<Channel>>>;

    async fn get_channel(&self, keytag: &Keytag) -> DbResult<Option<Channel>>;

    async fn get_all(&self) -> DbResult<BTreeMap<Keytag, Channel>>;

    async fn update_squash(&self, keytag: &Keytag, squash: Squash) -> DbResult<Channel>;

    async fn append_locked(&self, keytag: &Keytag, locked: Locked) -> DbResult<Channel>;

    async fn unlock(&self, keytag: &Keytag, secret: Secret) -> DbResult<Channel>;
}
