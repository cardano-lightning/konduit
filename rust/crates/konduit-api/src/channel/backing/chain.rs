use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::{
    BackingUtxo, NonEmpty, Succession,
    cardano::{BlockHeight, OutputReference},
};

// ---------------------------------------------------------------------------
// Chain — a lineage of BackingUtxos, Live or Lost
// ---------------------------------------------------------------------------

/// A lineage of backing UTXOs.
///
/// `Live`  — last is currently observed; contributes to effective backing.
/// `Lost`  — last has vanished (spent or rolled back, indistinguishable in
///           snapshot mode); retained as rollback fallback until the finality
///           window closes on the gap.
///
/// In snapshot mode chains are always singletons: lineage cannot be threaded
/// across snapshot boundaries without a `Succession`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum Chain {
    #[n(0)]
    Live {
        #[n(0)]
        links: NonEmpty<BackingUtxo>,
    },
    #[n(1)]
    Lost {
        #[n(1)]
        links: NonEmpty<BackingUtxo>,
        #[n(0)]
        lost_at: BlockHeight,
    },
}

impl Chain {
    // --- Constructors -------------------------------------------------------

    /// A fresh chain from a single newly-observed UTXO. Always `Live`.
    pub fn new(utxo: BackingUtxo) -> Self {
        Chain::Live {
            links: NonEmpty::singleton(utxo),
        }
    }

    /// A fresh chain from a single newly-observed UTXO. Always `Live`.
    pub fn links(&self) -> &NonEmpty<BackingUtxo> {
        match self {
            Chain::Live { links } | Chain::Lost { links, .. } => links,
        }
    }

    /// A fresh chain from a single newly-observed UTXO. Always `Live`.
    pub fn links_mut(&mut self) -> &mut NonEmpty<BackingUtxo> {
        match self {
            Chain::Live { links } | Chain::Lost { links, .. } => links,
        }
    }

    // --- Accessors --------------------------------------------------

    pub fn has(&self, output_reference: &OutputReference) -> bool {
        self.links()
            .iter()
            .any(|bu| bu.output_reference() == output_reference)
    }

    pub fn position(&self, output_reference: &OutputReference) -> Option<usize> {
        self.links()
            .iter()
            .position(|bu| bu.output_reference() == output_reference)
    }

    // --- State transitions --------------------------------------------------

    pub fn insert_after_and_split_off(
        &mut self,
        position: usize,
        bu: BackingUtxo,
    ) -> Option<Chain> {
        let tail = self.links_mut().tail_mut();
        let prev_descendents = tail.split_off(position);
        let lost_at = bu.block_height().clone();
        tail.push(bu);
        NonEmpty::try_from(prev_descendents)
            .ok()
            .map(|links| Chain::Lost { links, lost_at })
    }

    /// Mark this chain as Lost (tip no longer observed on-chain).
    pub fn lose(&mut self, now: BlockHeight) {
        if let Chain::Live { links } = self.to_owned() {
            *self = Chain::Lost {
                links,
                lost_at: now,
            }
        }
    }

    /// Mark this chain as Live again (tip reappeared — rollback of child).
    pub fn recover(self) -> Self {
        match self {
            Chain::Lost { links, .. } => Chain::Live { links },
            live => live,
        }
    }

    /// Parent must belong to chain, else Error.
    /// If parent is at tip, append child and return None,
    /// else split off existing decendents, and return Some(decendents)
    pub fn succeed(s: Succession) -> Result<Option<Chain>, ChainError> {
        // FIXME :: Use `insert_after_and_split_off`
        todo!()
    }

    // --- Accessors ----------------------------------------------------------

    pub fn is_live(&self) -> bool {
        matches!(self, Chain::Live { .. })
    }

    pub fn is_lost(&self) -> bool {
        matches!(self, Chain::Lost { .. })
    }

    pub fn last(&self) -> &BackingUtxo {
        self.links().last()
    }

    pub fn head(&self) -> &BackingUtxo {
        self.links().head()
    }

    // FIXME :: Chain no longer has current blockheight in context,
    // nor the adaptor block depth config
    // Thus it cannot determined status.
    //
    // /// The settled floor: the amount covered with zero rollback exposure.
    // /// This is the amount of the deepest settled UTXO in the lineage,
    // /// or zero if no UTXO has reached the finality window.
    // pub fn settled_floor(&self) -> u64 {
    //     if !self.is_live() {
    //         return 0;
    //     }
    //     self.links()
    //         .iter()
    //         .filter(|u| u.is_settled())
    //         .map(|u| u.amount)
    //         // The most recent settled link is the tightest floor.
    //         .last()
    //         .unwrap_or(0)
    // }

    // /// The tip amount — what this chain can back at the tip's depth.
    // pub fn tip_amount(&self) -> u64 {
    //     if self.is_live() { self.tip().amount } else { 0 }
    // }

    // /// The depth bucket of the tip.
    // pub fn tip_bucket(&self) -> Option<DepthBucket> {
    //     if self.is_live() {
    //         Some(self.tip().bucket())
    //     } else {
    //         None
    //     }
    // }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChainError {
    /// Attempted to extend a Lost chain.
    ExtendLost,
    /// The succession does not match the chain tip or the child.
    WitnessMismatch,
}

pub enum Succeeded {
    Append,
    Split(Chain),
    No(Succession),
}
