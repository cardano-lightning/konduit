//! Konduit authentication wire types.
//!
//! Defines the concrete [`Body`] implementation for the cobbl3 HMAC-BLAKE3
//! auth protocol.
//!
//! The server crate implements [`cobbl3::Verify`] for [`Body`] using cryptoxide.
//!
//! ## Headers
//! - `POST /a`            — initial auth (`Request<Body>`)
//! - `x-konduit-token`    — session token on subsequent requests (`Token`)

use minicbor::{Decode, Encode};

pub use cobbl3::{Mac, Request, Response};

pub const HEADER: &str = "x-konduit-token";

pub type Token = cobbl3::Token<Body>;

// ---------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------

/// The payload the client signs and the server MACs.
///
/// `key` and `tag` together identify a channel.
/// `ttl` is an absolute POSIX timestamp in milliseconds bounding the
/// validity window of the auth request, preventing signature replay.
///
/// Conversion to/from `konduit-data` types (`VerificationKey`, channel tag)
/// happens at the server layer — the wire crate treats both as plain bytes.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Body {
    /// Ed25519 verifying key (32 bytes). Identifies the channel owner.
    #[n(0)]
    pub key: [u8; 32],

    /// Channel tag. Opaque to the wire layer.
    #[n(1)]
    pub tag: Vec<u8>,

    /// Absolute expiry as POSIX milliseconds.
    #[n(2)]
    pub ttl: u64,
}

impl cobbl3::Body for Body {
    const DOMAIN: &'static str = "KONDUIT_AUTH";
}
