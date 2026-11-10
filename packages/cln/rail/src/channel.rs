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
use konduit_core::{Locked, Secret, SigningKey, Squash, SquashBody, Tag, Verified, VerifyingKey};

use crate::{
    Receipt,
    receipt::{self, WireReceipt},
};
use konduit_core::keytag;
use konduit_l2_dto::{
    pay::{Request as PayRequest, Response as PayOutcome},
    quote::{Request as QuoteRequest, Response as Quote},
    squash::Response as SquashStatus,
};
use konduit_rail::RESERVED_NAME as RESERVED;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MediaType {
    #[default]
    Json,
    Cbor,
}

/// Where a peer is reached, and nothing about any channel with them: a
/// key the rail knows, the url it answers on, and the codec to speak.
///
/// This is the lookup half. A key being known says the rail could pay
/// that peer if it had a channel; whether it has one is [`Entry`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownKey {
    /// Peer's key (ie sub_vkey).
    pub key: VerifyingKey,
    pub base_url: String,
    pub media_type: MediaType,
}

impl Default for KnownKey {
    fn default() -> Self {
        Self {
            key: SigningKey::from([255; 32]).verifying_key(),
            base_url: "http://localhost:7652".to_string(),
            media_type: Default::default(),
        }
    }
}

/// A channel the rail has: the tag names it, the key says whose known
/// url to reach it at. An entry whose key is not known is refused at
/// startup — see `config::Config::verify`.
///
/// Seeded from config today. Once this node can see the channels it is
/// the payer of on chain, an entry is what that view would hand over,
/// which is why it carries no url of its own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub key: VerifyingKey,
    pub tag: Tag,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            key: KnownKey::default().key,
            tag: Tag::from(hex::decode("deadbeef").unwrap()),
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
    /// A channel with `tag`, reached wherever `known` says.
    pub fn new(known: &KnownKey, tag: Tag, receipt: Receipt, auth: String) -> Self {
        let timeout = Some(Duration::from_secs(5));
        let client = Client::new(known.base_url.clone(), known.media_type.clone(), timeout);
        Self {
            tag,
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

    pub fn reveal(&self, lock: &konduit_core::Lock) -> Option<Secret> {
        self.receipt()
            .unlockeds()
            .find(|unlocked| unlocked.lock() == *lock)
            .map(|unlocked| *unlocked.secret())
    }

    pub fn apply_timeout(&self, now: konduit_core::Duration) -> Result<(), Error> {
        self.receipt().apply_timeout(now);
        Ok(())
    }

    pub fn apply_sync(&self, theirs: Receipt) -> Result<(), Error> {
        Ok(self.receipt().apply_sync(theirs)?)
    }

    pub fn has_pending_cheques(&self) -> bool {
        self.receipt().has_pending_cheques()
    }

    fn auth_header(&self) -> Box<dyn http_client::HeaderPolicy> {
        http_client::header_policy::Custom::new(keytag::HEADER, &self.auth).boxed()
    }

    /// Encodes an instruction in this channel's codec, for the rail at
    /// the other end to decode with the same one.
    pub fn encode_instruction<T>(&self, instruction: &T) -> Result<Vec<u8>, Error>
    where
        T: Serialize + minicbor::Encode<()>,
    {
        self.client.encode(instruction)
    }

    /// What `rail` charges to carry `amount`, on top of konduit's own.
    #[tracing::instrument(skip(self, instruction), err)]
    pub async fn quote_via(
        &self,
        rail: &str,
        amount: u64,
        instruction: Vec<u8>,
    ) -> Result<Quote, Error> {
        self.client
            .quote(rail, amount, instruction, self.auth_header())
            .await
    }

    /// Hands over a cheque, for `rail` to carry onward.
    #[tracing::instrument(skip(self, instruction), err)]
    pub async fn pay_via(
        &self,
        rail: &str,
        inbound: Locked,
        instruction: Vec<u8>,
    ) -> Result<PayOutcome, Error> {
        self.client
            .pay(rail, inbound, instruction, self.auth_header())
            .await
    }

    /// What the peer charges to be paid `amount`. The peer is the payee,
    /// so this is its own `/pay/konduit/quote`.
    pub async fn quote(&self, amount: u64) -> Result<Quote, Error> {
        self.quote_via(RESERVED, amount, Vec::new()).await
    }

    /// Hands the peer a cheque. Nothing for it to route onward.
    pub async fn pay(&self, inbound: Locked) -> Result<PayOutcome, Error> {
        self.pay_via(RESERVED, inbound, Vec::new()).await
    }

    /// Sends a signed squash, resolving whatever it covers.
    #[tracing::instrument(level = "debug", skip_all, err)]
    pub async fn squash(&self, squash: Squash) -> Result<SquashStatus, Error> {
        self.client.squash(squash, self.auth_header()).await
    }

    /// Fetches whatever receipt the peer currently has on file for this
    /// channel (`None` if it has never seen a squash from us). Touches no
    /// local state — callers decide what to do with it.
    #[tracing::instrument(level = "debug", skip_all, err)]
    pub async fn fetch_receipt(&self) -> Result<Option<WireReceipt>, Error> {
        self.client.receipt(self.auth_header()).await
    }
}

// FIXME :: This duplicates `konduit_client::Adaptor`, which speaks the
// same endpoints with the same keytag header. It should be rationalized
// onto that one — a rail is a konduit client pointed downstream, so the
// dependency runs this way round. Three things block it today:
//
//   - `Adaptor` is JSON only (`http_client::Client<T, codec::Json>`),
//     where a peer here may be configured for either codec;
//   - it answers in `anyhow`, flattening the status, where this rail has
//     to tell `Pending` from `OutboundRejected` from `RailUnavailable`
//     and must not do that by matching strings;
//   - `Adaptor::new` is eager and fallible (it fetches `/info`), so a
//     rail with several peers could not start while one was down, which
//     is worse than answering `rail-unavailable` per request.
//
// The receipt is not part of this: see `receipt.rs`.
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

    async fn get<Res>(
        &self,
        path: &str,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<Res, Error>
    where
        Res: for<'de> Deserialize<'de> + for<'b> minicbor::Decode<'b, ()>,
    {
        let headers = vec![auth];
        let res = match self {
            Client::Json(c) => c.get_with_headers::<Res>(path, headers).await?,
            Client::Cbor(c) => c.get_with_headers::<Res>(path, headers).await?,
        };
        Ok(res)
    }

    /// This client's codec, for an instruction that konduit carries but
    /// does not read.
    fn encode<T>(&self, instruction: &T) -> Result<Vec<u8>, Error>
    where
        T: Serialize + minicbor::Encode<()>,
    {
        match self {
            Client::Json(_) => serde_json::to_vec(instruction)
                .map_err(|err| Error::Transport(format!("encode instruction: {err}"))),
            Client::Cbor(_) => minicbor::to_vec(instruction)
                .map_err(|err| Error::Transport(format!("encode instruction: {err}"))),
        }
    }

    pub async fn quote(
        &self,
        rail: &str,
        amount: u64,
        instruction: Vec<u8>,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<Quote, Error> {
        let req = QuoteRequest {
            amount,
            instruction,
        };
        self.post(&format!("/pay/{rail}/quote"), &req, auth).await
    }

    pub async fn pay(
        &self,
        rail: &str,
        inbound: Locked,
        instruction: Vec<u8>,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<PayOutcome, Error> {
        let req = PayRequest {
            inbound,
            instruction,
        };
        self.post(&format!("/pay/{rail}/pay"), &req, auth).await
    }

    pub async fn squash(
        &self,
        squash: Squash,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<SquashStatus, Error> {
        self.post("/pay/konduit/squash", &squash, auth).await
    }

    pub async fn receipt(
        &self,
        auth: Box<dyn http_client::HeaderPolicy>,
    ) -> Result<Option<WireReceipt>, Error> {
        self.get("/pay/konduit/receipt", auth).await
    }
}
