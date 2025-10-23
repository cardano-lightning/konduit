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

    pub fn open(db_path: String) -> Result<Self, sled::Error> {
        Ok(Self {
            db: Arc::new(sled::open(db_path)?),
        })
    }
}

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
