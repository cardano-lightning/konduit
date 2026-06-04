use crate::Channel;
use cardano_sdk::Signature;
use konduit_data::{Keytag, L1Channel, Receipt};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;

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
pub struct UnlockedCheque {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub cheque_body: Vec<u8>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub signature: Signature,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub secret: Vec<u8>,
}
