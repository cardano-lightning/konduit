use anyhow::anyhow;
use async_trait::async_trait;
use futures::future::join_all;
use konduit_data::{Cheque, Duration, Squash};
use sled::{Db, IVec};
use std::{collections::BTreeMap, convert::Infallible, sync::Arc};

use crate::{
    db::{
        coiter_with_default::coiter_with_default,
        interface::{BackendError, UpdateSquashError},
    },
    l2_channel::L2Channel,
    models::{Keytag, L1Channel, TipBody, TipResponse},
};

use super::interface::{DbError, DbInterface};

#[derive(Debug, thiserror::Error)]
pub enum WithSledError<LogicError> {
    #[error("Sled Error : {0}")]
    Backend(SledBackendError),
    #[error("Other {0}")]
    Logic(LogicError),
}

impl<LogicError> From<WithSledError<LogicError>> for DbError<LogicError> {
    fn from(value: WithSledError<LogicError>) -> Self {
        match value {
            WithSledError::Backend(error) => DbError::Backend(BackendError::from(error)),
            WithSledError::Logic(error) => DbError::Logic(error),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SledBackendError {
    #[error("Sled Error : {0}")]
    Sled(sled::Error),
    #[error("Serde Error : {0}")]
    Serde(serde_json::Error),
}

impl From<sled::Error> for SledBackendError {
    fn from(e: sled::Error) -> Self {
        SledBackendError::Sled(e)
    }
}

impl From<serde_json::Error> for SledBackendError {
    fn from(e: serde_json::Error) -> Self {
        SledBackendError::Serde(e)
    }
}

impl From<SledBackendError> for BackendError {
    fn from(value: SledBackendError) -> Self {
        match value {
            SledBackendError::Sled(error) => BackendError::Other(error.to_string()),
            SledBackendError::Serde(error) => BackendError::Other(error.to_string()),
        }
    }
}

impl<T> From<SledBackendError> for WithSledError<T> {
    fn from(value: SledBackendError) -> Self {
        WithSledError::Backend(value)
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct SledArgs {
    /// The path to the database file
    #[clap(long, default_value = "konduit.db", env = crate::env::DB_PATH)]
    pub path: String,
}

pub struct WithSled {
    db: Arc<Db>,
}

impl TryFrom<&SledArgs> for WithSled {
    type Error = SledBackendError;

    fn try_from(value: &SledArgs) -> Result<Self, Self::Error> {
        let x = Self::open(value.path.clone())?;
        Ok(x)
    }
}

impl WithSled {
    pub fn open(db_path: String) -> Result<Self, sled::Error> {
        Ok(Self {
            db: Arc::new(sled::open(db_path)?),
        })
    }
}

pub fn get_channel_keys(db: &Db) -> Result<Vec<Keytag>, SledBackendError> {
    let range = [CHANNEL]..[CHANNEL_END];
    let res = db
        .range(range)
        .keys()
        .map(|result| result.map(|x| to_keytag(x.as_ref())))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(res)
}

async fn get_channel(db: &sled::Db, keytag: Keytag) -> Result<Option<L2Channel>, SledBackendError> {
    match db.get(to_db_key(keytag))? {
        Some(bytes) => {
            let channel = serde_json::from_slice(&bytes)?;
            Ok(Some(channel))
        }
        None => Ok(None),
    }
}

pub fn update_channel_db<F, E>(
    db: &Db,
    keytag: Keytag,
    update_fn: F,
) -> Result<L2Channel, WithSledError<E>>
where
    F: FnMut(Option<L2Channel>) -> Result<L2Channel, E>,
    E: std::fmt::Debug + 'static,
{
    let key = to_db_key(keytag);
    let update_fn_cell = std::cell::RefCell::new(update_fn);
    // FIXME :: ERROR HANDLING
    let transaction_result = db.transaction(move |tree: &sled::transaction::TransactionalTree| {
        let old_bytes_ivec: Option<IVec> = tree.get(&key)?;
        let old_channel: Option<L2Channel> = old_bytes_ivec
            .map(|bytes| serde_json::from_slice(bytes.as_ref()))
            .transpose()
            .unwrap(); //?;
        let new_channel: L2Channel = (update_fn_cell.borrow_mut())(old_channel).unwrap(); // .map_err(WithSledError::Logic)?;
        let new_bytes: Vec<u8> = serde_json::to_vec(&new_channel).unwrap(); //.map_err(SledBackendError::Serde)?;
        tree.insert(&*key, new_bytes)?;
        Ok(new_channel)
    });

    match transaction_result {
        Ok(new_channel) => Ok(new_channel),
        Err(sled::transaction::TransactionError::Abort(e)) => Err(e),
        Err(sled::transaction::TransactionError::Storage(e)) => {
            Err(WithSledError::Backend(SledBackendError::Sled(e)))
        }
    }
}

async fn update_from_l1(
    db: &sled::Db,
    keytag: Keytag,
    channels: Vec<L1Channel>,
) -> Result<L2Channel, WithSledError<Infallible>> {
    update_channel_db(
        db,
        keytag.clone(),
        |l2_channel: Option<L2Channel>| match l2_channel {
            Some(mut l2_channel) => {
                l2_channel.update_from_l1(channels.clone());
                Ok(l2_channel)
            }
            None => Ok(L2Channel::from_channels(keytag.clone(), channels.clone())),
        },
    )
}

#[allow(dead_code)]
async fn add_cheque(
    db: &sled::Db,
    keytag: Keytag,
    cheque: Cheque,
    // Acceptable timeout
    timeout: Duration,
) -> Result<L2Channel, WithSledError<anyhow::Error>> {
    update_channel_db(
        db,
        keytag,
        |l2_channel: Option<L2Channel>| match l2_channel {
            Some(mut l2_channel) => {
                l2_channel.add_cheque(cheque.clone(), timeout)?;
                Ok(l2_channel)
            }
            None => Err(anyhow!("No L2 channel found")),
        },
    )
}

async fn update_squash(
    db: &sled::Db,
    keytag: Keytag,
    squash: Squash,
) -> Result<L2Channel, WithSledError<UpdateSquashError>> {
    update_channel_db(
        db,
        keytag,
        |l2_channel: Option<L2Channel>| match l2_channel {
            Some(mut l2_channel) => {
                l2_channel
                    .update_squash(squash.clone())
                    .map_err(UpdateSquashError::Logic)?;
                Ok(l2_channel)
            }
            None => Err(UpdateSquashError::NotFound),
        },
    )
}

#[async_trait]
impl DbInterface for WithSled {
    async fn update_l1s(&self, tip: TipBody) -> Result<TipResponse, DbError<Infallible>> {
        let curr = get_channel_keys(self.db.as_ref()).map_err(WithSledError::Backend)?;
        let db = self.db.clone();
        let mut futures = vec![];
        coiter_with_default(tip.into_iter(), curr.into_iter(), |k, v| {
            let db = db.clone();
            futures.push(async move { update_from_l1(db.as_ref(), k, v).await });
        });
        join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // FIXME
        let r: BTreeMap<_, _> = BTreeMap::new();
        Ok(r)
    }

    async fn get_channel(&self, keytag: &Keytag) -> Result<Option<L2Channel>, DbError<Infallible>> {
        let db = self.db.as_ref();
        let res = get_channel(db, keytag.clone())
            .await
            .map_err(WithSledError::Backend)?;
        Ok(res)
    }

    async fn get_all(&self) -> Result<BTreeMap<Keytag, L2Channel>, DbError<Infallible>> {
        let ks = get_channel_keys(&self.db).map_err(WithSledError::Backend)?;
        let db = self.db.clone();
        let futures = ks.into_iter().map(async |k| {
            let db = db.clone();
            get_channel(&db, k.clone())
                .await
                .map_err(WithSledError::<Infallible>::Backend)
                .map(|ch| (k.clone(), ch.unwrap()))
        });
        let r = join_all(futures)
            .await
            .into_iter()
            .collect::<Result<_, _>>()?;
        Ok(r)
    }

    async fn update_squash(
        &self,
        keytag: Keytag,
        squash: Squash,
    ) -> Result<L2Channel, DbError<UpdateSquashError>> {
        let db = self.db.clone();
        let l2_channel = update_squash(&db, keytag, squash).await?;
        Ok(l2_channel)
    }
}

// START DB_KEYS
const CHANNEL: u8 = 10;
const CHANNEL_END: u8 = 19;

fn to_db_key(keytag: Keytag) -> Vec<u8> {
    std::iter::once(CHANNEL)
        .chain(keytag.0)
        .collect()
}

fn to_keytag(db_key: &[u8]) -> Keytag {
    Keytag(db_key[1..].to_vec())
}

// END OF DB_KEYS
