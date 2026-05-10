use crate::auth;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "quote-error"))]
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

    #[n(3)]
    #[error("insufficient funds: available={available}, required={required}")]
    Funds {
        #[n(0)]
        available: u64,
        #[n(1)]
        required: u64,
    },

    #[n(4)]
    #[error("route not found")]
    Route,

    /// Request payload exceeds the maximum size.
    #[n(5)]
    #[error("payload too large")]
    Size,

    #[n(6)]
    #[error("other: {0}")]
    Other(#[n(0)] String),

    /// Channel has no room for more cheques: too many unresolved payments or in-flight cheques.
    /// Client must submit a squash to free capacity before retrying.
    #[n(7)]
    #[error("insufficient capacity")]
    Capacity,

    /// A squash must be submitted before this operation can proceed.
    #[n(8)]
    #[error("squash required")]
    Squash,
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        match self {
            Self::Auth(e) => e.status_code(),
            Self::Limit(_) => 429,
            Self::Backing => 404,
            Self::Funds { .. } => 402,
            Self::Route => 404,
            Self::Size => 413,
            Self::Other(_) => 500,
            Self::Capacity => 429,
            Self::Squash => 428,
        }
    }
}
