//! Wire instructions carried opaquely by Konduit for the BLN rail.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Identifies an outbound Lightning payment when requesting a quote.
///
/// The signed invoice is retained verbatim and parsed by the rail using
/// `bln-sdk`; this keeps invoice parsing and validation in one place.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct QuoteInstruction {
    #[cfg_attr(feature = "cbor", n(0))]
    pub invoice: String,
}

/// Commits the outbound payment described by the signed Lightning invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(minicbor::Encode, minicbor::Decode))]
pub struct CommitInstruction {
    #[cfg_attr(feature = "cbor", n(0))]
    pub invoice: String,
}
