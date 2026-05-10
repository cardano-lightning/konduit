//! CL template: quote without a BOLT-11 invoice (Core Lightning style).
//!
//! The client specifies the payment parameters directly.
//! The lock (`r_hash`) will be taken from the cheque.
//! Using the template method allows a new class of payment failure:
//! user error (lock mismatch).

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "quote-cl-template-request"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Request {
    #[n(0)]
    pub amount: u64,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub payee: [u8; 32],
}

pub type Response = crate::common::channel::quote::Response;

pub type Error = crate::common::channel::quote::Error;
