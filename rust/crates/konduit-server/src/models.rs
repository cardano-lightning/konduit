use crate::Channel;
use bln_client::types::Invoice;
use cardano_tx_builder::Signature;
use konduit_data::{ChequeBody, L1Channel, Receipt, SquashProposal};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::BTreeMap;

pub use konduit_data::{Keytag, Stage};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    #[serde_as(as = "serde_with::hex::Hex")]
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

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QuoteBody {
    Simple(SimpleQuote),
    Bolt11(#[serde_as(as = "DisplayFromStr")] Invoice),
}

impl QuoteBody {
    pub fn amount_msat(&self) -> u64 {
        match self {
            QuoteBody::Simple(simple_quote) => simple_quote.amount_msat,
            QuoteBody::Bolt11(invoice) => invoice.amount_msat,
        }
    }

    pub fn payee(&self) -> [u8; 33] {
        match self {
            QuoteBody::Simple(simple_quote) => simple_quote.payee,
            QuoteBody::Bolt11(invoice) => invoice.payee_compressed,
        }
    }
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
    pub index: u64,
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
    pub cheque_body: ChequeBody,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
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
    Incomplete(SquashProposal),
}
