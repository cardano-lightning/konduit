//! Bln template : Allow the user to specify fields arbitrarily.
//!  
//! Quote without proof of invoice.
//!
//! If final `final_cltv` is None, then a defualt value is used.
//! The lock aka `r_hash` will be taken from the cheque.
//! Using the template method, allows a new c;ass pf payment failuer:
//! user error.

use bln_sdk::types::RouteHint;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

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
    #[n(3)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_cltv: Option<u64>,
}

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
