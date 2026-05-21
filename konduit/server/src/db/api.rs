use std::collections::BTreeMap;

use async_trait::async_trait;
use konduit_channel::{Channel, Error as ChannelError};
use konduit_data::Keytag;

#[async_trait]
pub trait Api: Send + Sync {
    async fn get_channel(&self, keytag: &Keytag) -> super::Result<Option<Channel>>;

    async fn get_all(&self) -> super::Result<BTreeMap<Keytag, Channel>>;

    async fn put_channel(&self, keytag: &Keytag, channel: Channel) -> super::Result<()>;

    /// Atomic read-mutate-write.
    /// Fails with `Logic::NoEntry` if the channel does not exist.
    async fn update_channel(
        &self,
        keytag: &Keytag,
        f: Box<dyn for<'a> FnOnce(&'a mut Channel) -> Result<(), ChannelError> + Send>,
    ) -> super::Result<Channel>;
}
