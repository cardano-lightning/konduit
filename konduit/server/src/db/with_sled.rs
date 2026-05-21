use async_trait::async_trait;
use konduit_channel::{Channel, Error as ChannelError};
use konduit_data::Keytag;
use sled::Db;
use std::{collections::BTreeMap, sync::Arc};

mod args;
pub use args::SledArgs as Args;

use super::{BackendError, Error, LogicError, api::Api};

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
    minicbor::to_vec(c).map_err(|e| BackendError::Serde(e.to_string()))
}

pub fn from_vec(v: &[u8]) -> Result<Channel, BackendError> {
    minicbor::decode(v).map_err(|e| BackendError::Serde(e.to_string()))
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

    fn get_channel_sync(&self, keytag: Keytag) -> super::Result<Option<Channel>> {
        match self.get_value(to_db_key(&keytag))? {
            Some(bytes) => {
                let channel = from_vec(&bytes)?;
                Ok(Some(channel))
            }
            None => Ok(None),
        }
    }

    fn put_channel_sync(&self, keytag: &Keytag, channel: Channel) -> super::Result<()> {
        let key = to_db_key(keytag);
        let bytes = into_vec(&channel)?;
        self.db
            .as_ref()
            .insert(key, bytes)
            .map_err(|e| Error::Backend(BackendError::Other(e.to_string())))?;
        Ok(())
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

    pub fn update_channel_sync(
        &self,
        keytag: &Keytag,
        update_fn: Box<dyn for<'a> FnOnce(&'a mut Channel) -> Result<(), ChannelError> + Send>,
    ) -> super::Result<Channel> {
        use std::sync::Mutex;
        let update_fn = Mutex::new(Some(update_fn));
        self.update_option_channel(keytag, move |opt| {
            let mut channel = opt.ok_or_else(|| LogicError::NoEntry(keytag.clone()))?;
            let f = update_fn
                .lock()
                .unwrap()
                .take()
                .expect("sled retried a FnOnce");
            f(&mut channel)?;
            Ok(channel)
        })
    }
}

#[async_trait]
impl Api for WithSled {
    async fn get_channel(&self, keytag: &Keytag) -> super::Result<Option<Channel>> {
        self.get_channel_sync(keytag.clone())
    }

    async fn get_all(&self) -> super::Result<BTreeMap<Keytag, Channel>> {
        let ks = self.channel_keys()?;
        let all = ks
            .into_iter()
            .map(|k| {
                // Justify unwrap :: we've only just grabbed all keys. It _must_ be a Some channel.
                self.get_channel_sync(k.clone())
                    .map(|ch| (k.clone(), ch.unwrap()))
            })
            .collect::<super::Result<_>>()?;
        Ok(all)
    }

    async fn put_channel(&self, keytag: &Keytag, channel: Channel) -> super::Result<()> {
        self.put_channel_sync(keytag, channel)
    }

    async fn update_channel(
        &self,
        keytag: &Keytag,
        f: Box<dyn for<'a> FnOnce(&'a mut Channel) -> Result<(), ChannelError> + Send>,
    ) -> super::Result<Channel> {
        self.update_channel_sync(keytag, f)
    }
}

// START DB_KEYS
const CHANNEL: u8 = 10;
const CHANNEL_END: u8 = 19;

fn to_db_key(keytag: &Keytag) -> Vec<u8> {
    std::iter::once(CHANNEL)
        .chain(keytag.as_ref().to_vec())
        .collect()
}

fn to_keytag(db_key: &[u8]) -> Keytag {
    Keytag::try_from(db_key[1..].to_vec()).expect("invalid keytag in database")
}
// END OF DB_KEYS
