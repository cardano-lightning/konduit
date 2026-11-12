use crate::Channel;
use cardano_sdk::Signature;
use konduit_core::{Keytag, Receipt};
use konduit_l1_dto::L1Channel;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnlockedCheque {
    #[serde(with = "konduit_core::serde::bytes")]
    pub cheque_body: Vec<u8>,
    #[serde(with = "konduit_core::serde::bytes")]
    pub signature: Signature,
    #[serde(with = "konduit_core::serde::bytes")]
    pub secret: Vec<u8>,
}
