<<<<<<< HEAD
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Constants {
=======
use std::collections::BTreeMap;

pub use konduit_data::Keytag;
use konduit_data::MixedReceipt;
pub use konduit_data::Stage;
use serde::{Deserialize, Serialize};

use crate::l2_channel::L2Channel;

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub fee: u64,
>>>>>>> e3cb13e (Updates to konduit data.)
    #[serde(with = "hex")]
    pub adaptor_key: [u8; 32],
    pub close_period: u64,
}

<<<<<<< HEAD
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteBody {
    #[serde(with = "hex")]
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
=======
pub type TipBody = BTreeMap<Keytag, Vec<L1Channel>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L1Channel {
    pub stage: Stage,
    pub amount: u64,
}

pub fn mk_data() -> TipBody {
    let vec = vec![
        (
            Keytag(
                hex::decode(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef00000000",
                )
                .unwrap(),
            ),
            vec![L1Channel {
                stage: Stage::Opened(0),
                amount: 1000000,
            }],
        ),
        (
            Keytag(
                hex::decode(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef00000001",
                )
                .unwrap(),
            ),
            vec![L1Channel {
                stage: Stage::Opened(0),
                amount: 1000000,
            }],
        ),
    ];
    vec.into_iter().collect()
}

pub type TipResponse = BTreeMap<Keytag, TipResult>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TipResult {
    New,
    MixedReceipt(MixedReceipt),
    Ended,
}

pub type ShowResponse = BTreeMap<Keytag, L2Channel>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Secrets(Vec<[u8; 32]>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteBody {
>>>>>>> e3cb13e (Updates to konduit data.)
    pub invoice: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteResponse {
    pub amount: u64,
    pub timeout: u64,
    #[serde(with = "hex")]
    pub lock: [u8; 32],
    #[serde(with = "hex")]
    pub recipient: [u8; 33],
    pub amount_msat: u64,
    #[serde(with = "hex")]
    pub payment_addr: [u8; 32],
    pub routing_fee: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayBody {
    #[serde(with = "hex")]
<<<<<<< HEAD
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
    #[serde(with = "hex")]
=======
>>>>>>> e3cb13e (Updates to konduit data.)
    pub cheque_body: Vec<u8>,
    #[serde(with = "hex")]
    pub signature: [u8; 64],
    #[serde(with = "hex")]
    pub recipient: [u8; 33],
    pub amount_msat: u64,
    #[serde(with = "hex")]
    pub payment_addr: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnlockedCheque {
    #[serde(with = "hex")]
    pub cheque_body: Vec<u8>,
    #[serde(with = "hex")]
    pub signature: [u8; 64],
    #[serde(with = "hex")]
    pub secret: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    #[serde(with = "hex")]
    pub squash_body: Vec<u8>,
    #[serde(with = "hex")]
    pub signature: [u8; 64],
    pub unlockeds: Vec<UnlockedCheque>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<Vec<u64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
<<<<<<< HEAD
pub struct SquashBody {
    #[serde(with = "hex")]
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
    #[serde(with = "hex")]
    pub squash_body: Vec<u8>,
    #[serde(with = "hex")]
    pub signature: [u8; 64],
=======
pub enum SquashResponse {
    Complete,
    Incomplete(IncompleteSquashResponse),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IncompleteSquashResponse {
    pub mixed_receipt: MixedReceipt,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire: Option<Vec<u64>>,
>>>>>>> e3cb13e (Updates to konduit data.)
}
