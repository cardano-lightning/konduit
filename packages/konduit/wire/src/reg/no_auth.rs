//! Register with no auth.
//!
//! The server responds without verification.
//! No funds at risk, but potential leaking of `/state`,
//! and risk of spamming.
//!
//! Keytage

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;
use std::str::FromStr;

/// Header
pub const SCHEME: &str = "None";
pub type Credential = Keytag;

/// Request
pub type Body = super::Body<Keytag>;

/// Response
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response();

#[derive(ProblemDetail)]
pub enum Error {
    #[problem(delegate)]
    Common(super::CommonError),
}

/// Keytag bytes carried in [`HEADER`].
///
/// Encodes `key || tag` — the server splits and interprets them.
/// `Display` encodes to base64url (no padding); `FromStr` decodes.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Encode, Decode)]
#[cbor(transparent)]
#[serde(transparent)]
pub struct Keytag(
    #[serde_as(as = "serde_with::hex::Hex")]
    #[n(0)]
    pub Vec<u8>,
);

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
