<<<<<<< HEAD
use actix_web::web;
use async_trait::async_trait;
use sled::Db;
use std::sync::Arc;

use crate::models::{Constants, PayBody, QuoteBody, QuoteResponse, Receipt, SquashBody};

use super::{DbError, DbInterface, channel_key};

pub struct DbSled {
    db: Arc<Db>,
}

impl DbSled {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

=======
use anyhow::anyhow;
use async_trait::async_trait;
use futures::future::join_all;
use konduit_data::{Cheque, Duration, Squash};
use sled::{Db, IVec};
use std::{collections::BTreeMap, convert::Infallible, sync::Arc};

use crate::{
    db::{
        coiter_with_default::coiter_with_default,
        interface::{BackendError, GetChannelError, UpdateError},
    },
    l2_channel::{self, L2Channel, L2ChannelUpdateSquashError},
    models::{Keytag, L1Channel, ShowResponse, SquashResponse, TipBody, TipResponse},
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
            SledBackendError::Sled(error) => {
                UpdateError::Backend(BackendError::Other(error.to_string()))
            }
            SledBackendError::Serde(error) => {
                UpdateError::Backend(BackendError::Other(error.to_string()))
            }
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
    #[clap(long, default_value = "konduit.db", env = "KONDUIT_DB_PATH")]
    pub path: String,
}

pub struct WithSled {
    db: Arc<Db>,
}

impl TryFrom<SledArgs> for WithSled {
    type Error = SledBackendError;

    fn try_from(value: SledArgs) -> Result<Self, Self::Error> {
        let x = Self::open(value.path)?;
        Ok(x)
    }
}

impl WithSled {
>>>>>>> e3cb13e (Updates to konduit data.)
    pub fn open(db_path: String) -> Result<Self, sled::Error> {
        Ok(Self {
            db: Arc::new(sled::open(db_path)?),
        })
    }
}

<<<<<<< HEAD
#[async_trait]
impl DbInterface for DbSled {
    async fn init(&self, constants: &Constants) -> Result<(), DbError> {
        let db = self.db.clone();
        // Create owned copies for the async block
        let adaptor_key = constants.adaptor_key;
        let close_period_bytes = constants.close_period.to_be_bytes();

        // Use a transaction as we are writing two keys atomically
        match web::block(move || {
            db.insert(b"constants:adaptor_key", adaptor_key.to_vec())?;
            db.insert(b"constants:close_period", close_period_bytes.to_vec())?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    // --- Getters ---

    async fn get_constants(&self) -> Result<Constants, DbError> {
        let db = self.db.clone();
        // Use web::block to move blocking sled calls to a thread pool.
        match web::block(move || {
            let adaptor_key_bytes = db
                .get(b"constants:adaptor_key")?
                .ok_or_else(|| DbError::NotFound("constants:adaptor_key".to_string()))?;
            let close_period_bytes = db
                .get(b"constants:close_period")?
                .ok_or_else(|| DbError::NotFound("constants:close_period".to_string()))?;

            let adaptor_key: [u8; 32] =
                adaptor_key_bytes
                    .to_vec()
                    .try_into()
                    .map_err(|e: Vec<u8>| {
                        DbError::InvalidData(format!("key wrong length: {}", e.len()))
                    })?;
            let close_period: u64 =
                u64::from_be_bytes(close_period_bytes.to_vec().try_into().map_err(
                    |e: Vec<u8>| {
                        DbError::InvalidData(format!("close_period wrong length: {}", e.len()))
                    },
                )?);

            Ok(Constants {
                adaptor_key,
                close_period,
            })
        })
        .await
        {
            Ok(Ok(constants)) => Ok(constants),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn get_quote_response(
        &self,
        key: &[u8; 32],
        tag: &[u8],
    ) -> Result<QuoteResponse, DbError> {
        let db = self.db.clone();
        let db_key = channel_key(key, tag, "quote_response");
        match web::block(move || {
            let bytes = db
                .get(db_key)?
                .ok_or_else(|| DbError::NotFound("quote_response".to_string()))?;
            let response = serde_json::from_slice(&bytes)?;
            Ok(response)
        })
        .await
        {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn get_receipt(&self, key: &[u8; 32], tag: &[u8]) -> Result<Receipt, DbError> {
        let db = self.db.clone();
        let db_key = channel_key(key, tag, "receipt");
        match web::block(move || {
            let bytes = db
                .get(db_key)?
                .ok_or_else(|| DbError::NotFound("receipt".to_string()))?;
            let receipt = serde_json::from_slice(&bytes)?;
            Ok(receipt)
        })
        .await
        {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    // --- Putters ---

    async fn put_quote_request(&self, request: &QuoteBody) -> Result<(), DbError> {
        let db = self.db.clone();
        let db_key = channel_key(&request.consumer_key, &request.tag, "quote_request");
        let bytes = serde_json::to_vec(request)?; // Serialize outside block

        match web::block(move || {
            db.insert(db_key, bytes)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn put_pay_request(&self, request: &PayBody) -> Result<(), DbError> {
        let db = self.db.clone();
        let db_key = channel_key(&request.consumer_key, &request.tag, "pay_request");
        let bytes = serde_json::to_vec(request)?;

        match web::block(move || {
            db.insert(db_key, bytes)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn put_squash_request(&self, request: &SquashBody) -> Result<(), DbError> {
        let db = self.db.clone();
        let db_key = channel_key(&request.consumer_key, &request.tag, "squash_request");
        let bytes = serde_json::to_vec(request)?;

        match web::block(move || {
            db.insert(db_key, bytes)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn put_quote_response(
        &self,
        key: &[u8; 32],
        tag: &[u8],
        response: &QuoteResponse,
    ) -> Result<(), DbError> {
        let db = self.db.clone();
        let db_key = channel_key(key, tag, "quote_response");
        let bytes = serde_json::to_vec(response)?;

        match web::block(move || {
            db.insert(db_key, bytes)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }

    async fn put_receipt(
        &self,
        key: &[u8; 32],
        tag: &[u8],
        receipt: &Receipt,
    ) -> Result<(), DbError> {
        let db = self.db.clone();
        let db_key = channel_key(key, tag, "receipt");
        let bytes = serde_json::to_vec(receipt)?;

        match web::block(move || {
            db.insert(db_key, bytes)?;
            Ok(())
        })
        .await
        {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(db_err)) => Err(db_err),
            Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
        }
    }
}
=======
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
    E: 'static,
{
    let key = to_db_key(keytag);
    let update_fn_cell = std::cell::RefCell::new(update_fn);
    let transaction_result = db.transaction(move |tree: &sled::transaction::TransactionalTree| {
        let old_bytes_ivec: Option<IVec> = tree.get(&key)?;
        let old_channel: Option<L2Channel> = old_bytes_ivec
            .map(|bytes| serde_json::from_slice(bytes.as_ref()))
            .transpose()?;
        let new_channel: L2Channel =
            (update_fn_cell.borrow_mut())(old_channel).map_err(WithSledError::Logic)?;
        let new_bytes: Vec<u8> = serde_json::to_vec(&new_channel).map_err(SledBackend::Serde);
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
                l2_channel.add_cheque(cheque.clone(), timeout.clone())?;
                Ok(l2_channel)
            }
            None => Err(anyhow!("No L2 channel found")),
        },
    )
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateSquashError {
    #[error("Channel not found")]
    NotFound,
    #[error("Other {0}")]
    Logic(L2ChannelUpdateSquashError),
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
                l2_channel.update_squash(squash.clone())?;
                Ok(l2_channel)
            }
            None => return Err(UpdateSquashError::NotFound),
        },
    )
}

#[async_trait]
impl DbInterface for WithSled {
    async fn sync_tip(&self, tip: TipBody) -> Result<TipResponse, DbError<Infallible>> {
        let curr = get_channel_keys(self.db.as_ref())?;
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

    async fn get_channel(&self, keytag: &Keytag) -> Result<L2Channel, DbError> {
        let db = self.db.as_ref();
        get_channel(db, keytag.clone()).await
    }

    async fn show(&self) -> Result<ShowResponse, DbError> {
        let ks = get_channel_keys(&*self.db)?;
        let db = self.db.clone();
        let futures = ks.into_iter().map(async |k| {
            let db = db.clone();
            get_channel(&db, k.clone()).await.map(|ch| (k.clone(), ch))
        });
        let r = join_all(futures)
            .await
            .into_iter()
            .collect::<Result<BTreeMap<Keytag, L2Channel>, DbError>>()
            .unwrap();
        Ok(r)
    }

    async fn squash(&self, keytag: Keytag, squash: Squash) -> Result<SquashResponse, DbError> {
        let db = self.db.clone();
        let l2_channel = update_squash(&db, keytag, squash).await;
        if is_complete {
        } else {
        }
    }

    // async fn get_quote_response(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    // ) -> Result<QuoteResponse, DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(key, tag, "quote_response");
    //     match web::block(move || {
    //         let bytes = db
    //             .get(db_key)?
    //             .ok_or_else(|| DbError::NotFound("quote_response".to_string()))?;
    //         let response = serde_json::from_slice(&bytes)?;
    //         Ok(response)
    //     })
    //     .await
    //     {
    //         Ok(Ok(resp)) => Ok(resp),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // async fn get_receipt(&self, key: &[u8; 32], tag: &[u8]) -> Result<Receipt, DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(key, tag, "receipt");
    //     match web::block(move || {
    //         let bytes = db
    //             .get(db_key)?
    //             .ok_or_else(|| DbError::NotFound("receipt".to_string()))?;
    //         let receipt = serde_json::from_slice(&bytes)?;
    //         Ok(receipt)
    //     })
    //     .await
    //     {
    //         Ok(Ok(resp)) => Ok(resp),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // // --- Putters ---

    // async fn put_quote_request(&self, request: &QuoteBody) -> Result<(), DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(&request.consumer_key, &request.tag, "quote_request");
    //     let bytes = serde_json::to_vec(request)?; // Serialize outside block

    //     match web::block(move || {
    //         db.insert(db_key, bytes)?;
    //         Ok(())
    //     })
    //     .await
    //     {
    //         Ok(Ok(_)) => Ok(()),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // async fn put_pay_request(&self, request: &PayBody) -> Result<(), DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(&request.consumer_key, &request.tag, "pay_request");
    //     let bytes = serde_json::to_vec(request)?;

    //     match web::block(move || {
    //         db.insert(db_key, bytes)?;
    //         Ok(())
    //     })
    //     .await
    //     {
    //         Ok(Ok(_)) => Ok(()),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // async fn put_squash_request(&self, request: &SquashBody) -> Result<(), DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(&request.consumer_key, &request.tag, "squash_request");
    //     let bytes = serde_json::to_vec(request)?;

    //     match web::block(move || {
    //         db.insert(db_key, bytes)?;
    //         Ok(())
    //     })
    //     .await
    //     {
    //         Ok(Ok(_)) => Ok(()),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // async fn put_quote_response(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    //     response: &QuoteResponse,
    // ) -> Result<(), DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(key, tag, "quote_response");
    //     let bytes = serde_json::to_vec(response)?;

    //     match web::block(move || {
    //         db.insert(db_key, bytes)?;
    //         Ok(())
    //     })
    //     .await
    //     {
    //         Ok(Ok(_)) => Ok(()),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }

    // async fn put_receipt(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    //     receipt: &Receipt,
    // ) -> Result<(), DbError> {
    //     let db = self.db.clone();
    //     let db_key = channel_key(key, tag, "receipt");
    //     let bytes = serde_json::to_vec(receipt)?;

    //     match web::block(move || {
    //         db.insert(db_key, bytes)?;
    //         Ok(())
    //     })
    //     .await
    //     {
    //         Ok(Ok(_)) => Ok(()),
    //         Ok(Err(db_err)) => Err(db_err),
    //         Err(join_err) => Err(DbError::TaskJoin(join_err.to_string())),
    //     }
    // }
}

// START DB_KEYS
const CHANNEL: u8 = 10;
const CHANNEL_END: u8 = 19;

fn to_db_key(keytag: Keytag) -> Vec<u8> {
    std::iter::once(CHANNEL)
        .chain(keytag.0.into_iter())
        .collect()
}

fn to_keytag(db_key: &[u8]) -> Keytag {
    Keytag(db_key[1..].to_vec())
}

// END OF DB_KEYS
>>>>>>> e3cb13e (Updates to konduit data.)
