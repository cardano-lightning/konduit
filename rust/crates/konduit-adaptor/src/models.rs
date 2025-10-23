use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Constants {
    #[serde(with = "hex")]
    pub adaptor_key: [u8; 32],
    pub close_period: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteBody {
    #[serde(with = "hex")]
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
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
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
    #[serde(with = "hex")]
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
pub struct SquashBody {
    #[serde(with = "hex")]
    pub consumer_key: [u8; 32],
    #[serde(with = "hex")]
    pub tag: Vec<u8>,
    #[serde(with = "hex")]
    pub squash_body: Vec<u8>,
    #[serde(with = "hex")]
    pub signature: [u8; 64],
}
