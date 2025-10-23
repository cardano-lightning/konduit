use async_trait::async_trait;
use thiserror::Error;

use crate::models::{Constants, PayBody, QuoteBody, QuoteResponse, Receipt, SquashBody};

pub mod with_sled;
pub use with_sled::DbSled;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Sled database error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Serialization/Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Invalid data in DB: {0}")]
    InvalidData(String),
    #[error("Task execution error: {0}")]
    TaskJoin(String),
}

/// Defines the interface for all database interactions.
/// Handlers will use this trait, not the concrete `sled::Db`.
#[async_trait]
pub trait DbInterface: Send + Sync {
    async fn init(&self, constants: &Constants) -> Result<(), DbError>;
    async fn get_constants(&self) -> Result<Constants, DbError>;
    async fn get_quote_response(
        &self,
        key: &[u8; 32],
        tag: &[u8],
    ) -> Result<QuoteResponse, DbError>;
    async fn get_receipt(&self, key: &[u8; 32], tag: &[u8]) -> Result<Receipt, DbError>;
    async fn put_quote_request(&self, request: &QuoteBody) -> Result<(), DbError>;
    async fn put_pay_request(&self, request: &PayBody) -> Result<(), DbError>;
    async fn put_squash_request(&self, request: &SquashBody) -> Result<(), DbError>;
    async fn put_quote_response(
        &self,
        key: &[u8; 32],
        tag: &[u8],
        response: &QuoteResponse,
    ) -> Result<(), DbError>;
    async fn put_receipt(
        &self,
        key: &[u8; 32],
        tag: &[u8],
        receipt: &Receipt,
    ) -> Result<(), DbError>;
}

/// A helper function to create prefixed, unambiguous channel keys.
/// Format: "channel:<consumer_key_hex>|<tag_hex>|<field_name>"
fn channel_key(key: &[u8; 32], tag: &[u8], field_id: &str) -> Vec<u8> {
    let mut db_key = Vec::new();
    db_key.extend_from_slice(b"channel:");
    db_key.extend_from_slice(&hex::encode(key).as_bytes());
    db_key.push(b'|');
    db_key.extend_from_slice(&hex::encode(tag).as_bytes());
    db_key.push(b'|');
    db_key.extend_from_slice(field_id.as_bytes());
    db_key
}

pub fn open(db_path: String) -> Result<impl DbInterface, DbError> {
    DbSled::open(db_path).map_err(|err| DbError::Sled(err))
}
