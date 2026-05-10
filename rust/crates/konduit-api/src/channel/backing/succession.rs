use super::{BackingUtxo, cardano::OutputReference};

/// A succession informs that lineage is to be extended.
/// Produced either by the chain indexer (full lineage mode) or by the
/// server observing its own submitted TX appear on-chain.
pub struct Succession {
    /// The UTXO that was spent.
    parent: OutputReference,
    /// The UTXO produced as its child.
    child: BackingUtxo,
}

impl Succession {
    pub fn new(parent: OutputReference, child: BackingUtxo) -> Self {
        Self { parent, child }
    }

    pub fn parent(&self) -> &OutputReference {
        &self.parent
    }

    pub fn child(&self) -> &BackingUtxo {
        &self.child
    }
}
