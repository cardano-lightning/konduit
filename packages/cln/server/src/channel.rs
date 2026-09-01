//! Channel client
//!
//! A single client's client. Codec is fixed at construction time from config,
//! so callers never need to know or care which one it is.
//!
//! Multiple channels to the same peer are not supported in this design.
//! To do so would require a more elaborate querying.

use std::{sync::Mutex, time::Duration};

use serde::{Deserialize, Serialize};

use http_client::codec;
use konduit_data::{Locked, Secret, SigningKey, Squash, SquashBody, Tag, Verified, VerifyingKey};

use crate::{
    Receipt,
    receipt::{self, WireReceipt},
    wire,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MediaType {
    #[default]
    Json,
    Cbor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tag: Tag,
    /// Receiver's key (ie sub_vkey).
    pub key: VerifyingKey,
    /// Receiver's
    pub base_url: String,
    pub media_type: MediaType,
}

impl Default for Config {
    fn default() -> Self {
        let key = SigningKey::from([255; 32]).verifying_key();
        let tag = Tag::from(hex::decode("deadbeef").unwrap());
        Self {
            tag,
            key,
            base_url: "http://localhost:7652".to_string(),
            media_type: Default::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("transport: {0}")]
    Transport(String),
    #[error("receipt: {0}")]
    Receipt(#[from] receipt::Error),
}

impl<T, E, D> From<http_client::ClientError<T, E, D>> for Error
where
    T: std::fmt::Display,
    E: std::fmt::Display,
    D: std::fmt::Display,
{
    fn from(value: http_client::ClientError<T, E, D>) -> Self {
        use http_client::ClientError::*;
        let detail = match &value {
            Transport(e) => format!("transport: {e}"),
            Encode(e) => format!("encode: {e}"),
            Decode(e) => format!("decode: {e}"),
            Http(e) => format!("http (request build): {e}"),
            Status(code, msg) => format!("status {code}: {msg:?}"),
            BuilderCorrupted => "builder corrupted".to_string(),
        };
        Self::Transport(detail)
    }
}

pub struct Channel {
    tag: Tag,
    /// There are short sections of logic where we need to mutate the receipt.
    receipt: Mutex<Receipt>,
    client: Client,
    /// Value sent under `wire::auth::HEADER` on every request.
    auth: String,
}

impl Channel {
    pub fn new(config: &Config, receipt: Receipt, auth: String) -> Self {
        let timeout = Some(Duration::from_secs(5));
        let client = Client::new(config.base_url.clone(), config.media_type.clone(), timeout);
        Self {
            tag: config.tag.clone(),
            receipt: Mutex::new(receipt),
            client,
            auth,
        }
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    fn receipt(&self) -> std::sync::MutexGuard<'_, Receipt> {
        self.receipt.lock().expect("receipt state poisoned")
    }

    pub fn wire_receipt(&self) -> WireReceipt {
        WireReceipt::from(self.receipt().clone())
    }

    pub fn propose_index(&self) -> u64 {
        self.receipt().propose_index()
    }

    pub fn propose_squash_body(&self) -> Result<SquashBody, Error> {
        Ok(self.receipt().propose_squash_body()?)
    }

    pub fn maybe_propose_squash_body(&self) -> Result<Option<SquashBody>, Error> {
        Ok(self.receipt().maybe_propose_squash_body()?)
    }

    pub fn apply_locked(&self, locked: Locked<Verified>) -> Result<(), Error> {
        Ok(self.receipt().apply_locked(locked)?)
    }

    pub fn apply_squash(&self, squash: Squash<Verified>) -> Result<(), Error> {
        Ok(self.receipt().apply_squash(squash)?)
    }

    pub fn apply_secret(&self, secret: Secret) -> Result<(), Error> {
        Ok(self.receipt().apply_secret(secret)?)
    }

    pub fn apply_timeout(&self, now: konduit_data::Duration) -> Result<(), Error> {
        Ok(self.receipt().apply_timeout(now))
    }

    pub fn apply_sync(&self, theirs: Receipt) -> Result<(), Error> {
        Ok(self.receipt().apply_sync(theirs)?)
    }

    fn auth_header(&self) -> Box<dyn http_client::HeaderPolicy> {
        http_client::header_policy::Custom::new(wire::auth::HEADER, &self.auth).boxed()
    }

    pub async fn quote(&self, req: &wire::quote::Request) -> Result<wire::quote::Response, Error> {
        self.client.quote(req, self.auth_header()).await
    }

    pub async fn commit(
        &self,
        req: &wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        self.client.commit(req, self.auth_header()).await
    }

    pub async fn sync(&self, req: &wire::sync::Request) -> Result<wire::sync::Response, Error> {
        self.client.sync(req, self.auth_header()).await
    }
}

enum Client {
    Json(http_client::Client<http_client::transport::Reqwest, codec::Json>),
    Cbor(http_client::Client<http_client::transport::Reqwest, codec::Cbor>),
}

impl Client {
    pub fn new(base_url: String, media_type: MediaType, timeout: Option<Duration>) -> Self {
        let transport = http_client::transport::Reqwest::new(timeout);
        match media_type {
            MediaType::Json => {
                Client::Json(http_client::Client::new(transport, codec::Json, base_url))
            }
            MediaType::Cbor => {
                Client::Cbor(http_client::Client::new(transport, codec::Cbor, base_url))
            }
        }
    }

    async fn post<Req, Res>(
        &self,
        path: &str,
        req: &Req,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<Res, Error>
    where
        Req: Serialize + minicbor::Encode<()>,
        Res: for<'de> Deserialize<'de> + for<'b> minicbor::Decode<'b, ()>,
    {
        let headers = vec![auth];
        let res = match self {
            Client::Json(c) => c.post_with_headers::<Req, Res>(path, req, headers).await?,
            Client::Cbor(c) => c.post_with_headers::<Req, Res>(path, req, headers).await?,
        };
        Ok(res)
    }

    pub async fn quote(
        &self,
        req: &wire::quote::Request,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<wire::quote::Response, Error> {
        self.post(wire::quote::PATH, req, auth).await
    }

    pub async fn commit(
        &self,
        req: &wire::commit::Request,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<wire::commit::Response, Error> {
        self.post(wire::commit::PATH, req, auth).await
    }

    pub async fn sync(
        &self,
        req: &wire::sync::Request,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<wire::sync::Response, Error> {
        self.post(wire::sync::PATH, req, auth).await
    }
}
