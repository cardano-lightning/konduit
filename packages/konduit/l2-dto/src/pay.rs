use konduit_core::{Locked, Secret};
#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub type Error = crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Request {
    /// The inbound commitment: konduit verifies it, and takes the lock
    /// and the ceiling from it.
    #[cfg_attr(feature = "cbor", n(0))]
    pub inbound: Locked,
    /// The rail's own commit instruction, encoded.
    #[cfg_attr(feature = "serde", serde(with = "konduit_core::serde::bytes", default))]
    #[cfg_attr(feature = "cbor", n(1))]
    pub instruction: Vec<u8>,
}

/// What a consumer is told of a payment. A failure is a problem
/// response, not an outcome here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub enum Response {
    /// Paid, and here is the proof. Resolve the inbound leg by squash.
    #[cfg_attr(feature = "cbor", n(0))]
    Success(#[cfg_attr(feature = "cbor", n(0))] Secret),
    /// Committed and unresolved. Poll squash status; the secret arrives
    /// as an unlocked cheque in the proposal.
    #[cfg_attr(feature = "cbor", n(1))]
    Pending,
}

pub type Result = std::result::Result<Response, Error>;

impl Response {
    pub fn secret(&self) -> Option<Secret> {
        match self {
            Self::Success(secret) => Some(*secret),
            Self::Pending => None,
        }
    }
}
