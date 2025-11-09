use std::{collections::BTreeMap, convert::Infallible};

use async_trait::async_trait;
use konduit_data::{Cheque, Keytag, Secret, Squash};

use crate::{
    l2_channel::{
        L2Channel, L2ChannelInsertChequeError, L2ChannelUnlockError, L2ChannelUpdateSquashError,
    },
    models::{TipBody, TipResponse},
};

#[derive(Debug, thiserror::Error)]
pub enum DbError<LogicError> {
    #[error("BackendError : {0}")]
    Backend(BackendError),
    #[error("Logic : {0}")]
    Logic(LogicError),
}

impl From<DbError<Infallible>> for DbError<UpdateSquashError> {
    fn from(value: DbError<Infallible>) -> Self {
        match value {
            DbError::Backend(error) => DbError::Backend(error),
            DbError::Logic(_) => panic!("Impossible"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Other : {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateSquashError {
    #[error("Channel not found")]
    NotFound,
    #[error("Logic : {0}")]
    Logic(L2ChannelUpdateSquashError),
}

#[derive(Debug, thiserror::Error)]
pub enum InsertChequeError {
    #[error("Channel not found")]
    NotFound,
    #[error("Logic : {0}")]
    Logic(L2ChannelInsertChequeError),
}

#[derive(Debug, thiserror::Error)]
pub enum UnlockError {
    #[error("Channel not found")]
    NotFound,
    #[error("Logic : {0}")]
    Logic(L2ChannelUnlockError),
}

#[async_trait]
pub trait DbInterface: Send + Sync {
    /// Get funds available in
    async fn update_l1s(&self, tip: TipBody) -> Result<TipResponse, DbError<Infallible>>;

    async fn get_channel(&self, keytag: &Keytag) -> Result<Option<L2Channel>, DbError<Infallible>>;

    async fn get_all(&self) -> Result<BTreeMap<Keytag, L2Channel>, DbError<Infallible>>;

    async fn update_squash(
        &self,
        keytag: &Keytag,
        squash: Squash,
    ) -> Result<L2Channel, DbError<UpdateSquashError>>;

    async fn insert_cheque(
        &self,
        keytag: &Keytag,
        cheque: Cheque,
    ) -> Result<L2Channel, DbError<InsertChequeError>>;

    async fn unlock(
        &self,
        keytag: &Keytag,
        secret: Secret,
    ) -> Result<L2Channel, DbError<UnlockError>>;
}
