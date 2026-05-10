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
use crate::common::channel::SquashProposal;
use konduit_channel::DepthBucket;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_response(val: &Response) -> Response {
        let bytes = minicbor::to_vec(val).expect("encode failed");
        minicbor::decode(&bytes).expect("decode failed")
    }

    fn roundtrip_error(val: &Error) -> Error {
        let bytes = minicbor::to_vec(val).expect("encode failed");
        minicbor::decode(&bytes).expect("decode failed")
    }

    #[test]
    fn response_no_backing_no_proposal() {
        let orig = Response {
            backing: None,
            squash_proposal: None,
        };
        let decoded = roundtrip_response(&orig);
        assert!(decoded.backing.is_none());
        assert!(decoded.squash_proposal.is_none());
    }

    #[test]
    fn response_with_backing() {
        let orig = Response {
            backing: Some(BackingView {
                amount: 5_000_000,
                bucket: DepthBucket::Settled,
            }),
            squash_proposal: None,
        };
        let decoded = roundtrip_response(&orig);
        let bv = decoded.backing.unwrap();
        assert_eq!(bv.amount, 5_000_000);
        assert_eq!(bv.bucket, DepthBucket::Settled);
    }

    #[test]
    fn backing_view_all_buckets() {
        use DepthBucket::*;
        for bucket in [Unconfirmed, Shallow, Probable, Deep, Settled] {
            let orig = BackingView { amount: 1, bucket };
            let bytes = minicbor::to_vec(&orig).unwrap();
            let decoded: BackingView = minicbor::decode(&bytes).unwrap();
            assert_eq!(decoded.bucket, bucket);
        }
    }

    #[test]
    fn error_auth_roundtrip() {
        use crate::auth::pop;
        let orig = Error::Auth(pop::Error::BadSignature);
        let decoded = roundtrip_error(&orig);
        assert!(matches!(decoded, Error::Auth(pop::Error::BadSignature)));
    }

    #[test]
    fn error_limit_roundtrip() {
        let orig = Error::Limit("too fast".into());
        let decoded = roundtrip_error(&orig);
        assert!(matches!(decoded, Error::Limit(s) if s == "too fast"));
    }

    #[test]
    fn error_backing_roundtrip() {
        let orig = Error::Backing;
        let decoded = roundtrip_error(&orig);
        assert!(matches!(decoded, Error::Backing));
    }
}
