//! No-auth: channel identity asserted directly via header.
//!
//! The client sends a base64url-encoded keytag on every request.
//! The server trusts it without verification — only suitable for
//! deployments where transport-level security is sufficient.
//!
//! ## Header
//! `x-konduit-keytag: <base64url(key || tag)>`

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use minicbor::{Decode, Encode};
use std::fmt;
use std::str::FromStr;

pub const HEADER: &str = "x-konduit-keytag";

/// Opaque keytag bytes carried in [`HEADER`].
///
/// Encodes `key || tag` — the server splits and interprets them.
/// `Display` encodes to base64url (no padding); `FromStr` decodes.
/// Todo : remove this from konduit-data
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cbor(transparent)]
pub struct Keytag(#[n(0)] pub Vec<u8>);

impl fmt::Display for Keytag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&URL_SAFE_NO_PAD.encode(&self.0))
    }
}

impl FromStr for Keytag {
    type Err = base64::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        URL_SAFE_NO_PAD.decode(s).map(Keytag)
    }
}
