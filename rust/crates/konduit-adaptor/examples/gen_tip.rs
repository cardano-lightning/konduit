use futures::future::join_all;
use konduit_adaptor::{
    l2_channel::L2Channel,
    models::{Info, L1Channel, QuoteBody},
};
use konduit_data::{
    Cheque, ChequeBody, Duration, Indexes, Keytag, Lock, MixedReceipt, Secret, Squash, SquashBody,
    Stage, Tag,
};
use proptest::prelude::*;
use proptest::test_runner::TestRunner;
use std::{cell::RefCell, collections::BTreeMap, fmt::Display};

use cardano_tx_builder::{
    PlutusData, SigningKey, VerificationKey,
    cbor::{self, ToCbor},
};
use serde_json::json;

pub struct Client {
    base_url: String,
    client: reqwest::Client,
}

fn group_maps<X: Ord, Y>(input: impl Iterator<Item = BTreeMap<X, Y>>) -> BTreeMap<X, Vec<Y>> {
    input
        .flat_map(|map| map.into_iter()) // Flatten all maps into a single iterator of (X, Y) pairs
        .fold(BTreeMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_insert_with(Vec::new).push(value);
            acc // Return the accumulator for the next iteration
        })
}

impl Default for Client {
    fn default() -> Self {
        let base_url = "http://localhost:4445".to_string();
        let mut client_builder = reqwest::Client::builder();
        let client = client_builder.build().unwrap();
        Self { base_url, client }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    Serde(serde_json::Error),
    ApiError(u16, String),
    Any(String),
}

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
        ClientError::Serde(e)
    }
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(value: reqwest::Error) -> Self {
        ClientError::Any(value.to_string())
    }
}

impl Client {
    pub async fn info(&self) -> Result<Info, ClientError> {
        let info: Info = serde_json::from_value(self.get("info").await?)?;
        Ok(info)
    }

    pub async fn admin_tip(&self, tip: BTreeMap<Keytag, Vec<L1Channel>>) {
        let result = self.post("admin/tip", tip).await;
        println!("{:?}", result);
    }

    pub async fn admin_show(&self) {
        let result = self.get("admin/show").await;
        println!("{:?}", result);
    }

    pub async fn ch_squash(&self, kv: &Keytag, squash: Squash) {
        let result = self
            .ch_bytes("ch/squash", kv, PlutusData::from(squash).to_cbor())
            .await;
        println!("{:?}", result);
    }

    pub async fn ch_quote(&self, kv: &Keytag, quote_body: QuoteBody) {
        let result = self.ch_json("ch/quote", kv, quote_body).await;
        println!("{:?}", result);
    }

    /// Helper to handle API errors
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> Result<serde_json::Value, ClientError> {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }

        let status = response.status().as_u16();
        let message = response.text().await.unwrap_or("No message".to_string());

        Err(ClientError::ApiError(status, message))
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        if response.status().is_success() {
            self.handle_response(response).await
        } else {
            panic!("{:?}", response.text().await?);
        }
    }

    async fn ch_bytes(
        &self,
        path: &str,
        keytag: &Keytag,
        body: Vec<u8>,
    ) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .header("konduit", hex::encode(&keytag.0))
            .body(body)
            .send()
            .await?;
        self.handle_response(response).await
    }

    async fn ch_json(
        &self,
        path: &str,
        keytag: &Keytag,
        body: impl serde::Serialize,
    ) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header("konduit", hex::encode(&keytag.0))
            .json(&body)
            .send()
            .await?;
        self.handle_response(response).await
    }

    async fn post(
        &self,
        path: &str,
        body: impl serde::Serialize,
    ) -> Result<serde_json::Value, ClientError> {
        let url = format!("{}/{}", self.base_url, path);
        let response = self.client.post(&url).json(&body).send().await?;
        self.handle_response(response).await
    }
}

fn adaptor_vkey() -> VerificationKey {
    let adaptor_skey = SigningKey::from([0; 32]);
    let adaptor_vkey = VerificationKey::from(&adaptor_skey);
    adaptor_vkey
}

#[derive(Debug, thiserror::Error)]
pub enum ConsumerError {
    #[error("No Channel")]
    NoChannel(String),
    #[error("Bad Mixed Receipt")]
    BadMixedReceipt,
    #[error("Uninitialized")]
    Uninitialized,
    #[error("Logic Error {0}")]
    Logic(String),
}

#[derive(Debug, Clone)]
struct Consumer {
    signing_key: SigningKey,
    amount: u64,
    channels: BTreeMap<Tag, L2Channel>,
}

impl Consumer {
    pub fn new(seed: u8) -> Self {
        let mut inner = [0; 32];
        inner[0] = seed;
        Self {
            signing_key: SigningKey::from(inner),
            amount: 100000000,
            channels: BTreeMap::new(),
        }
    }

    pub fn verification_key(&self) -> VerificationKey {
        VerificationKey::from(&self.signing_key)
    }

    pub fn new_channel(&mut self, tag: Tag) {
        let keytag = Keytag::new(self.verification_key(), tag.clone());
        let l1_channel = L1Channel {
            stage: Stage::Opened(0),
            amount: 100000,
        };
        let kt = keytag.clone();
        let squash_body = SquashBody {
            amount: 0,
            index: 0,
            exclude: Indexes(vec![]),
        };
        let squash = Squash::make(&self.signing_key, &tag, squash_body);
        let mixed_receipt = MixedReceipt::new(squash.clone(), vec![]).unwrap();
        let l2_channel = L2Channel {
            keytag,
            l1_channel: Some(l1_channel),
            mixed_receipt: Some(mixed_receipt),
            is_served: true,
        };

        self.channels.insert(tag, l2_channel);
    }

    /// We cannot call this without either setting up the "merchant" and / or running against a
    /// mock BLN network.
    pub fn new_cheque(
        &mut self,
        tag: &Tag,
        amount: u64,
        timeout: Duration,
        lock: Lock,
    ) -> Result<Cheque, ConsumerError> {
        let Some(channel) = self.channels.get_mut(tag) else {
            return Err(ConsumerError::NoChannel(hex::encode(&tag.0)));
        };
        let Some(ref mut mixed_receipt) = channel.mixed_receipt else {
            return Err(ConsumerError::Uninitialized);
        };
        let index = mixed_receipt.max_index() + 1;
        let cheque_body = ChequeBody::new(index, amount, timeout, lock);
        let cheque = Cheque::make(&self.signing_key, tag, cheque_body);
        mixed_receipt
            .insert(cheque.clone())
            .map_err(|err| ConsumerError::Logic(err.to_string()))?;
        Ok(cheque)
    }

    pub fn update(
        &mut self,
        tag: &Tag,
        mixed_receipt: MixedReceipt,
    ) -> Result<Squash, ConsumerError> {
        let vkey = self.verification_key();
        let Some(channel) = self.channels.get_mut(tag) else {
            return Err(ConsumerError::NoChannel(hex::encode(&tag.0)));
        };

        if !mixed_receipt.verify_components(&vkey, tag) {
            return Err(ConsumerError::BadMixedReceipt);
        }
        let Ok(squash_body) = mixed_receipt.make_squash_body() else {
            return Err(ConsumerError::BadMixedReceipt);
        };
        let squash = Squash::make(&self.signing_key, tag, squash_body);
        let Some(ref mut current) = channel.mixed_receipt else {
            return Err(ConsumerError::Uninitialized);
        };
        current
            .update(squash.clone())
            .map_err(|err| ConsumerError::Logic(err.to_string()))?;
        Ok(squash)
    }

    pub fn l1_channels(&self) -> BTreeMap<Keytag, L1Channel> {
        self.channels
            .iter()
            .filter_map(|(k, v)| {
                v.l1_channel
                    .clone()
                    .map(|ch| (Keytag::new(self.verification_key(), k.clone()), ch))
            })
            .collect::<BTreeMap<Keytag, L1Channel>>()
    }
}

/// Attempts to use proptest. For now this is not helpful.
fn consumer_strategy() -> impl Strategy<Value = Consumer> {
    let signing_key_strategy = any::<[u8; 32]>();
    let amount_strategy = 0..1000000000000u64;
    (signing_key_strategy, amount_strategy).prop_map(|(signing_key, amount)| Consumer {
        signing_key: SigningKey::from(signing_key),
        amount,
        channels: BTreeMap::new(),
    })

    // HOW TO USE:
    // let mut runner = TestRunner::default();
    // let strategy = consumer_strategy();
    // // Proptest runner does not permit mutable action.
    // // This is a work around.
    // let generated_sample = RefCell::new(Vec::<Consumer>::new());
    // let _ = runner.run(&strategy, |x| {
    //     generated_sample.borrow_mut().push(x.clone());
    //     Ok(()).
    // });
}

async fn lifecycle() {
    let mut consumer = Consumer::new(0);
    consumer.new_channel(Tag(b"my_channel!!".to_vec()));
    let tip = group_maps([consumer.l1_channels().clone()].into_iter());

    let client = Client::default();
    let info = client.info().await.expect("No info");
    let _ = client.admin_tip(tip).await;
    // let _ = client.admin_show().await;
    let futures = consumer
        .channels
        .iter()
        .map(|(_k, v)| client.ch_squash(&v.keytag, v.mixed_receipt.clone().unwrap().squash));
    let r = join_all(futures).await.into_iter().collect::<Vec<_>>();

    let invoice = "lntb10u1p5sn3mspp5my0azm6ax3gzaz5nkfrhrj6535a52zxpsgqzltv8725atte8ge6qdqqcqzzsxqyz5vqsp5fsv5m4whjhr48t4tf4an4y5k9ytl83vxqhtd5elqyvdwqeqpeyhq9qxpqysgqzw04z5sezf7arhrcfa3qgp7g90j4dl34yskmdwwxv7c2arwzmhn35z4yt77pp2hnksz0f5gk2ke4y4cs93x3whq4yyytffff2menz7sp9kekj6".to_string();
    let quote_body: QuoteBody = QuoteBody::Bolt11(invoice);
    let futures = consumer
        .channels
        .iter()
        .map(|(_k, v)| client.ch_quote(&v.keytag, quote_body.clone()));
    let r = join_all(futures).await.into_iter().collect::<Vec<_>>();
    println!("{:?}", r);
}

#[tokio::main]
async fn main() {
    lifecycle().await
}
