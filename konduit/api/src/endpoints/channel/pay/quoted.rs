use crate::auth;
use konduit_data::Cheque;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "pay-quoted-request", inner = "cheque"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Request {
    #[cbor(n(0), with = "konduit_data::cbor_with::plutus_data")]
    pub cheque: Cheque,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "pay-quoted-response"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Response {
    #[n(0)]
    Inflight,
    #[n(1)]
    Ok,
    #[n(2)]
    Ko,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "pay-quoted-error"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Error {
    #[n(0)]
    #[error(transparent)]
    Auth(#[n(0)] auth::pop::Error),

    #[n(1)]
    #[error("rate limit exceeded: {0}")]
    Limit(#[n(0)] String),

    #[n(2)]
    #[error("no backing")]
    Backing,

    /// Cheque is malformed or its signature does not verify.
    #[n(3)]
    #[error("bad cheque")]
    Cheque,

    /// The cheque's lock does not match the invoice's payment hash.
    #[n(4)]
    #[error("lock does not match invoice payment hash")]
    Lock,

    #[n(5)]
    #[error("insufficient funds: available={available}, required={required}")]
    Funds {
        #[n(0)]
        available: u64,
        #[n(1)]
        required: u64,
    },

    /// The cheque timeout does not leave enough margin for the adaptor to act.
    #[n(6)]
    #[error("timeout too short: minimum {minimum_ms}ms required")]
    Timeout {
        #[n(0)]
        minimum_ms: u64,
    },

    /// The lightning payment could not be routed.
    #[n(7)]
    #[error("routing failed")]
    Routing,

    /// Channel has no room for more cheques: too many unresolved payments or in-flight cheques.
    /// Client must submit a squash to free capacity before retrying.
    #[n(8)]
    #[error("insufficient capacity")]
    Capacity,

    /// A squash must be submitted before this operation can proceed.
    #[n(9)]
    #[error("squash required")]
    Squash,
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        match self {
            Self::Auth(e) => e.status_code(),
            Self::Limit(_) => 429,
            Self::Backing => 404,
            Self::Cheque => 400,
            Self::Lock => 422,
            Self::Funds { .. } => 402,
            Self::Timeout { .. } => 400,
            Self::Routing => 502,
            Self::Capacity => 429,
            Self::Squash => 428,
        }
    }
}
