use async_trait::async_trait;
use sled::Db;
use std::{collections::BTreeMap, sync::Arc};

use konduit_data::{Keytag, Locked, Secret, Squash};

mod args;
pub use args::SledArgs as Args;

use crate::{Channel, ChannelError, channel::Retainer};

use super::{BackendError, Error, LogicError, api::Api, coiter_with_default::coiter_with_default};

impl From<sled::Error> for BackendError {
    fn from(e: sled::Error) -> Self {
        Self::Other(e.to_string())
    }
}

pub struct WithSled {
    db: Arc<Db>,
}

impl TryFrom<&Args> for WithSled {
    type Error = BackendError;

    fn try_from(value: &Args) -> Result<Self, Self::Error> {
        let x = Self::open(value.path.clone())?;
        Ok(x)
    }
}

pub fn into_vec(c: &Channel) -> Result<Vec<u8>, BackendError> {
    let v = postcard::to_stdvec(c)?;
    Ok(v)
}

pub fn from_vec(v: &[u8]) -> Result<Channel, BackendError> {
    let c = postcard::from_bytes(v)?;
    Ok(c)
}

impl WithSled {
    pub fn open(db_path: String) -> Result<Self, BackendError> {
        Ok(Self {
            db: Arc::new(sled::open(db_path)?),
        })
    }

    pub fn open_temporary() -> Result<Self, BackendError> {
        Ok(Self {
            db: Arc::new(sled::Config::new().temporary(true).open()?),
        })
    }

    pub fn channel_keys(&self) -> Result<Vec<Keytag>, BackendError> {
        let range = [CHANNEL]..[CHANNEL_END];
        let res = self
            .db
            .as_ref()
            .range(range)
            .keys()
            .map(|result| result.map(|x| to_keytag(x.as_ref())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    fn get_value(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, BackendError> {
        let b = self.db.as_ref().get(key)?;
        Ok(b.map(|v| v.to_vec()))
    }

    fn get_channel(&self, keytag: Keytag) -> super::Result<Option<Channel>> {
        match self.get_value(to_db_key(&keytag))? {
            Some(bytes) => {
                let channel = from_vec(&bytes)?;
                Ok(Some(channel))
            }
            None => Ok(None),
        }
    }

    pub fn update_option_channel<F>(&self, keytag: &Keytag, update_fn: F) -> super::Result<Channel>
    where
        F: Fn(Option<Channel>) -> Result<Channel, LogicError>,
    {
        let abort_backend =
            |err| sled::transaction::ConflictableTransactionError::Abort(Error::Backend(err));
        let abort_logic =
            |err| sled::transaction::ConflictableTransactionError::Abort(Error::Logic(err));

        let key = to_db_key(keytag);
        let result: Result<Channel, sled::transaction::TransactionError<Error>> =
            self.db.transaction(move |tree| {
                let key = key.clone();
                let old_channel = tree
                    .get(&key)?
                    .map(|bytes| from_vec(bytes.as_ref()))
                    .transpose()
                    .map_err(abort_backend)?;
                let new_channel = update_fn(old_channel).map_err(abort_logic)?;
                let new_bytes = into_vec(&new_channel).map_err(abort_backend)?;
                tree.insert(key.as_slice(), new_bytes)?;
                Ok(new_channel)
            });

        match result {
            Ok(new_channel) => Ok(new_channel),
            Err(sled::transaction::TransactionError::Abort(e)) => Err(e),
            Err(sled::transaction::TransactionError::Storage(e)) => {
                Err(Error::Backend(BackendError::Other(e.to_string())))
            }
        }
    }

    pub fn update_channel<F, T>(&self, keytag: &Keytag, update_fn: F) -> super::Result<Channel>
    where
        F: Fn(&mut Channel) -> Result<T, ChannelError>,
    {
        let wrap = move |opt: Option<Channel>| {
            let mut channel = opt.ok_or_else(|| LogicError::NoEntry(keytag.clone()))?;
            update_fn(&mut channel)?;
            Ok(channel)
        };
        self.update_option_channel(keytag, wrap)
    }

    fn one_update_retainers(
        &self,
        keytag: &Keytag,
        retainers: Vec<Retainer>,
    ) -> super::Result<Channel> {
        self.update_option_channel(keytag, move |opt| {
            let mut channel = opt.unwrap_or_else(|| Channel::new(keytag.clone()));
            channel.update_retainer(retainers.clone())?;
            Ok(channel)
        })
    }
}

#[async_trait]
impl Api for WithSled {
    async fn update_retainers(
        &self,
        retainers: BTreeMap<Keytag, Vec<Retainer>>,
    ) -> super::Result<BTreeMap<Keytag, Result<Channel, ChannelError>>> {
        let curr = self.channel_keys()?;
        let mut futures = Vec::new();
        coiter_with_default(retainers.into_iter(), curr.into_iter(), |k, v| {
            futures.push(async move { (k.clone(), self.one_update_retainers(&k, v)) });
        });
        let results = futures::future::join_all(futures).await;
        let mut res = BTreeMap::new();
        for (key, update_result) in results {
            match update_result {
                Ok(channel) => {
                    res.insert(key, Ok(channel));
                }
                Err(Error::Logic(LogicError::Channel(ce))) => {
                    res.insert(key, Err(ce));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(res)
    }

    async fn get_channel(&self, keytag: &Keytag) -> super::Result<Option<Channel>> {
        let res = self.get_channel(keytag.clone())?;
        Ok(res)
    }

    async fn get_all(&self) -> super::Result<BTreeMap<Keytag, Channel>> {
        let ks = self.channel_keys()?;
        let all = ks
            .into_iter()
            .map(|k| {
                // Justify unwrap :: we've only just grabbed all keys. It _must_ be a Some channel.
                self.get_channel(k.clone())
                    .map(|ch| (k.clone(), ch.unwrap()))
            })
            .collect::<super::Result<_>>()?;
        Ok(all)
    }

    async fn update_squash(&self, keytag: &Keytag, squash: Squash) -> super::Result<Channel> {
        self.update_channel(keytag, |c: &mut Channel| {
            c.update_squash(squash.clone())?;
            Ok(())
        })
    }

    async fn append_locked(&self, keytag: &Keytag, locked: Locked) -> super::Result<Channel> {
        self.update_channel(keytag, |c: &mut Channel| {
            c.append_locked(locked.clone())?;
            Ok(())
        })
    }

    async fn unlock(&self, keytag: &Keytag, secret: Secret) -> super::Result<Channel> {
        self.update_channel(keytag, |c: &mut Channel| {
            c.unlock(secret.clone())?;
            Ok(())
        })
    }
}

// START DB_KEYS
const CHANNEL: u8 = 10;
const CHANNEL_END: u8 = 19;

fn to_db_key(keytag: &Keytag) -> Vec<u8> {
    std::iter::once(CHANNEL).chain(keytag.0.clone()).collect()
}

fn to_keytag(db_key: &[u8]) -> Keytag {
    Keytag(db_key[1..].to_vec())
}
// END OF DB_KEYS
