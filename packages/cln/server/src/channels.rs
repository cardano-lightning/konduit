use crate::{Channel, channel};
use konduit_data::VerifyingKey;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

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

pub struct Channels {
    channels: BTreeMap<VerifyingKey, Arc<Channel>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a peer: {0:?}")]
    NoRoute(VerifyingKey),
}

impl Channels {
    pub fn new() -> Self {
        Self {
            channels: Default::default(),
        }
    }

    pub fn insert(&mut self, key: VerifyingKey, channel: Channel) -> Option<Arc<Channel>> {
        self.channels.insert(key, Arc::new(channel))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VerifyingKey, &Arc<Channel>)> {
        self.channels.iter()
    }

    pub fn get(&self, peer: &VerifyingKey) -> Result<&Arc<Channel>, Error> {
        self.channels.get(peer).ok_or(Error::NoRoute(peer.clone()))
    }
}
