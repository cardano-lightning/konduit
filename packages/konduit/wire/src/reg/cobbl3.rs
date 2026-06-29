//! Defines the concrete [`Body`] implementation for the cobbl3 HMAC-BLAKE3
//! auth protocol.
//!
//! The server crate implements [`cobbl3::Verify`] for [`Body`] using cryptoxide.
//!
//! Beware! There is mild divergence between what is a "token" between here and Cobbl3.

use minicbor::{Decode, Encode};

use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};

/// Header
pub const SCHEME: &str = "Cobbl3";
pub type Credential = cobbl3::Token<TokenBody>;

/// Request
pub type Body = super::Body<cobbl3::Request<TokenBody>>;

/// Response
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Response(#[n(0)] pub cobbl3::Response);

#[derive(Debug, Clone, ProblemDetail)]
pub enum Error {
    #[problem(delegate)]
    Common(super::CommonError),
    #[problem(delegate)]
    Cobbl3(cobbl3::Error),
}

// ---------------------------------------------------------------------------
// Token Body
// ---------------------------------------------------------------------------

/// The payload the client signs and the server MACs.
///
/// `key` and `tag` together identify a channel.
/// `ttl` is an absolute POSIX timestamp in milliseconds bounding the
/// validity window of the auth request.
///
/// Conversion to/from `konduit-data` types (`VerificationKey`, channel tag)
/// happens at the server layer — the wire crate treats both as plain bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct TokenBody {
    /// Consumer key
    #[n(0)]
    pub key: [u8; 32],
    /// Channel tag
    #[n(1)]
    pub tag: Vec<u8>,
    /// Expiry as absolute POSIX milliseconds.
    #[n(2)]
    pub ttl: u64,
}

impl cobbl3::Body for TokenBody {
    const DOMAIN: &'static str = "KONDUIT_AUTH";
}
