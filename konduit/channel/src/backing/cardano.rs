// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Number of blocks since a UTXO was first observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BlockDepth(pub u64);

/// Absolute block height.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Encode,
    Decode,
)]
#[repr(transparent)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct BlockHeight(#[n(0)] pub u64);

/// Unique reference to a UTXO: (transaction hash, output index).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct OutputReference {
    #[n(0)]
    pub transaction_id: [u8; 32],
    #[n(1)]
    pub output_index: u64,
}

// -------------------------------------------------------------------- Building

impl OutputReference {
    pub fn new(transaction_id: [u8; 32], output_index: u64) -> Self {
        Self {
            transaction_id,
            output_index,
        }
    }
}

// ------------------------------------------------------------------ Inspecting

impl OutputReference {
    pub fn transaction_id(&self) -> &[u8; 32] {
        &self.transaction_id
    }

    pub fn output_index(&self) -> u64 {
        self.output_index
    }
}
