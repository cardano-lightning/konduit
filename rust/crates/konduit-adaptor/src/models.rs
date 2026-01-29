use std::collections::BTreeMap;

use cardano_tx_builder::Signature;
pub use konduit_data::Keytag;
pub use konduit_data::Stage;
use konduit_data::{Cheque, Receipt};
use konduit_data::{L1Channel, Locked};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Channel;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    #[serde(with = "hex")]
    pub adaptor_key: [u8; 32],
    pub close_period: u64,
    pub fee: u64,
    pub max_tag_length: usize,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub deployer_vkey: [u8; 32],
    #[serde_as(as = "serde_with::hex::Hex")]
    pub script_hash: [u8; 28],
}

pub type TipBody = BTreeMap<Keytag, Vec<L1Channel>>;

pub type TipResponse = BTreeMap<Keytag, TipResult>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TipResult {
    New,
    Receipt(Receipt),
    Ended,
}

pub type ShowResponse = BTreeMap<Keytag, Channel>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Secrets(Vec<[u8; 32]>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QuoteBody {
    Simple(SimpleQuote),
    Bolt11(String),
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimpleQuote {
    pub amount_msat: u64,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub payee: [u8; 33],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteResponse {
    pub amount: u64,
    pub relative_timeout: u64,
    // TODO (@waalge) TBD whether these fields are relevant.
    // #[serde(with = "hex")]
    // pub lock: [u8; 32],
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    pub routing_fee: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayBody {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub cheque_body: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: [u8; 64],
    pub invoice: String,
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    // pub final_cltv_delta: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnlockedCheque {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub cheque_body: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub secret: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SquashResponse {
    Complete,
    Incomplete(IncompleteSquashResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncompleteSquashResponse {
    pub receipt: Receipt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<Vec<u64>>,
}
