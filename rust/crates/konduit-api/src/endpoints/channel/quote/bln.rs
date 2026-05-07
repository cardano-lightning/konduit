use bln_sdk::types::RouteHint;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Quote without proof of invoice.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)]
    pub amount_msat: u64,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    pub payee: [u8; 33],
    #[n(2)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub route_hints: Vec<RouteHint>,
}

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
