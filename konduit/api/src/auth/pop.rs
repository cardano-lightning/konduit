//! Proof-of-Possession authentication headers.
//!
//! The client proves ownership of their Ed25519 private key by signing a
//! deterministic, domain-separated payload constructed as:
//!
//! ```text
//! payload = cbor.encode(["__KONDUIT_AUTH__", token, server_pubkey, keytag])
//! ```
//!
//! The `b"KONDUIT_AUTH"` prefix is a domain separator ensuring signatures
//! produced here cannot be replayed against any other CBOR payload the
//! user's key might sign elsewhere.

pub const FEATURE: &str = "auth.pop";

#[cfg(feature = "actix")]
pub mod actix;
mod common;
pub mod error;

pub use common::{DOMAIN, HEADER_KEYTAG, HEADER_SIGNATURE, Headers, to_bytes};
pub use error::Error;

// ./register
//
//     { keytag: Vec<u8>, token : [u8;32] , signature : Signature }
//
// ./channel/ <- lookup auth session tokens.
