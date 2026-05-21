use super::cardano::{BlockHeight, OutputReference};
// ---------------------------------------------------------------------------
// BackingUtxo — a single link in a chain
// ---------------------------------------------------------------------------

use konduit_data::Used;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct BackingUtxo {
    /// block in which the utxo is observed
    #[n(0)]
    block_height: BlockHeight,
    /// The output reference of the utxo
    #[n(1)]
    output_reference: OutputReference,
    /// The (effective) amount locked in the Utxo
    #[n(2)]
    amount: u64,
    /// Only valid Utxos in Opened stage are elligible backers.
    #[n(3)]
    subbed: u64,
    #[cbor(n(4), with = "konduit_data::cbor_with::vec_plutus_data")]
    useds: Vec<Used>,
}

impl BackingUtxo {
    pub fn new(
        block_height: BlockHeight,
        output_reference: OutputReference,
        amount: u64,
        subbed: u64,
        useds: Vec<Used>,
    ) -> Self {
        Self {
            block_height,
            output_reference,
            amount,
            subbed,
            useds,
        }
    }

    pub fn output_reference(&self) -> &OutputReference {
        &self.output_reference
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
    pub fn block_height(&self) -> &BlockHeight {
        &self.block_height
    }
    pub fn subbed(&self) -> u64 {
        self.subbed
    }
    pub fn useds(&self) -> &Vec<Used> {
        &self.useds
    }
}
