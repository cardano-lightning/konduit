//! The consumer-facing quote and pay bodies.
//!
//! Konduit's half of each is fixed — an amount, or the inbound cheque —
//! and the rest is the rail's instruction, which konduit carries and
//! does not parse. It travels as bytes in whichever codec the request
//! itself is in, so a rail deserializes it with that same codec and
//! konduit never has to know the shape.
//!
//! `/pay/konduit/…` takes the same bodies and ignores the instruction:
//! there is nothing downstream of konduit to instruct.

use konduit_data::{Locked, Secret};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct QuoteRequest {
    /// In the instance asset, before anyone's fee.
    #[n(0)]
    pub amount: u64,
    /// The rail's own quote instruction, encoded.
    #[serde(with = "hex::serde", default)]
    #[n(1)]
    pub instruction: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct PayRequest {
    /// The inbound commitment: konduit verifies it, and takes the lock
    /// and the ceiling from it.
    #[n(0)]
    pub inbound: Locked,
    /// The rail's own commit instruction, encoded.
    #[serde(with = "hex::serde", default)]
    #[n(1)]
    pub instruction: Vec<u8>,
}

/// What a consumer is told of a payment. A failure is a problem
/// response, not an outcome here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum PayOutcome {
    /// Paid, and here is the proof. Resolve the inbound leg by squash.
    #[n(0)]
    Success(#[n(0)] Secret),
    /// Committed and unresolved. Poll squash status; the secret arrives
    /// as an unlocked cheque in the proposal.
    #[n(1)]
    Pending,
}

impl PayOutcome {
    pub fn secret(&self) -> Option<Secret> {
        match self {
            Self::Success(secret) => Some(*secret),
            Self::Pending => None,
        }
    }
}
