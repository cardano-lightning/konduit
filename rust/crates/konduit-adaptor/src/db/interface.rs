use std::convert::Infallible;

use async_trait::async_trait;
use konduit_data::{Keytag, Squash};

use crate::{
    l2_channel::{L2Channel, L2ChannelUpdateSquashError},
    models::{ShowResponse, TipBody, TipResponse},
};

#[derive(Debug, thiserror::Error)]
pub enum DbError<LogicError> {
    #[error("BackendError : {0}")]
    Backend(BackendError),
    #[error("Logic : {0}")]
    Logic(LogicError),
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Other : {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SyncTipError {
    #[error("Other {0}")]
    Logic(L2ChannelUpdateSquashError),
}

#[derive(Debug, thiserror::Error)]
pub enum GetChannelError {
    #[error("Channel not found")]
    NotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum SquashError {
    #[error("Channel not found")]
    NotFound,
    #[error("Logic : {0}")]
    Logic(L2ChannelUpdateSquashError),
}

#[async_trait]
pub trait DbInterface: Send + Sync {
    /// Get funds available in
    async fn sync_tip(&self, tip: TipBody) -> Result<TipResponse, DbError<Infallible>>;

    async fn show(&self) -> Result<ShowResponse, DbError<Infallible>>;

    async fn get_channel(&self, keytag: &Keytag) -> Result<L2Channel, DbError<GetChannelError>>;

    async fn squash(
        &self,
        keytag: Keytag,
        squash: Squash,
    ) -> Result<L2Channel, DbError<SquashError>>;

    // Get funds available in
    // async fn get_available(&self, &keytag: Vec<u8>) -> Result<Constants, DbError>;

    // async fn get_constants(&self) -> Result<Constants, DbError>;
    // async fn get_quote_response(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    // ) -> Result<QuoteResponse, DbError>;
    // async fn get_receipt(&self, key: &[u8; 32], tag: &[u8]) -> Result<Receipt, DbError>;
    // async fn put_quote_request(&self, request: &QuoteBody) -> Result<(), DbError>;
    // async fn put_pay_request(&self, request: &PayBody) -> Result<(), DbError>;
    // async fn put_squash_request(&self, request: &SquashBody) -> Result<(), DbError>;
    // async fn put_quote_response(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    //     response: &QuoteResponse,
    // ) -> Result<(), DbError>;
    // async fn put_receipt(
    //     &self,
    //     key: &[u8; 32],
    //     tag: &[u8],
    //     receipt: &Receipt,
    // ) -> Result<(), DbError>;
}

// /// A helper function to create prefixed, unambiguous channel keys.
// /// Format: "channel:<consumer_key_hex>|<tag_hex>|<field_name>"
// fn channel_key(key: &[u8; 32], tag: &[u8], field_id: &str) -> Vec<u8> {
//     let mut db_key = Vec::new();
//     db_key.extend_from_slice(b"channel:");
//     db_key.extend_from_slice(&hex::encode(key).as_bytes());
//     db_key.push(b'|');
//     db_key.extend_from_slice(&hex::encode(tag).as_bytes());
//     db_key.push(b'|');
//     db_key.extend_from_slice(field_id.as_bytes());
//     db_key
// }
//
// pub fn open(db_path: String) -> Result<impl Interface, DbError> {
//     DbSled::open(db_path).map_err(|err| DbError::Sled(err))
// }
//
