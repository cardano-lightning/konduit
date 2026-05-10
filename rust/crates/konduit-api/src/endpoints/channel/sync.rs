//! `GET /channel/sync`
//!
//! Returns the server's current view of the channel.
//! Clients use this to recover local state and to determine what squash to sign next.
//!
//! Note: the full internal `Backing` (chain lineage, Lost chains, UTxO references,
//! subbed/useds accounting) is not disclosed. Clients receive a `BackingView` summary.
//!
//! `squash_proposal` is `None` until the client has submitted their first squash.
//! The client must submit a null squash (amount=0) before any `pay` is accepted.

use crate::auth;
use crate::channel::DepthBucket;
use konduit_data::SquashProposal;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Client-visible summary of the channel's on-chain backing.
/// The server's full `Backing` (chain lineage, Lost chains, UTxO references,
/// subbed/useds tracking) is not disclosed.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BackingView {
    /// Total lovelace locked in live chain tips.
    #[n(0)]
    pub amount: u64,
    /// Confirmation depth of the best live chain tip.
    /// Governs fee exposure and payment risk.
    #[n(1)]
    pub bucket: DepthBucket,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "sync-response"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Response {
    /// Summarised on-chain backing.
    /// `None` if the channel has not yet been funded on-chain.
    #[cbor(n(0), with = "cbor_with::nullable_same")]
    pub backing: Option<BackingView>,
    /// The next squash the server expects the client to sign, together with
    /// the in-flight cheques needed to reproduce it.
    /// `None` until the client has submitted their first (null) squash.
    #[cbor(n(1), with = "cbor_with::nullable_same")]
    #[cfg_attr(feature = "cddl", cddl(ty = "squash-proposal / null"))]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<Object>))]
    pub squash_proposal: Option<SquashProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "sync-error"))]
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
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        match self {
            Self::Auth(e) => e.status_code(),
            Self::Limit(_) => 429,
            Self::Backing => 404,
        }
    }
}
