//! BLN template: quote without a BOLT-11 invoice.
//!
//! The client specifies the payment parameters directly.
//! The lock (`r_hash`) will be taken from the cheque.
//! Using the template method allows a new class of payment failure:
//! user error (lock mismatch).
//!
//! If `final_cltv` is None, a server default is used.

use bln_sdk::types::RouteHint;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "quote-bln-template-request"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Request {
    #[n(0)]
    pub amount_msat: u64,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub payee: [u8; 33],
    #[n(2)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "cddl", cddl(ty = "[* route-hint]"))]
    pub route_hints: Vec<RouteHint>,
    #[n(3)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_cltv: Option<u64>,
}

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
