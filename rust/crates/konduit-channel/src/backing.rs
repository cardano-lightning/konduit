pub mod cardano;

mod succession;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
pub use succession::Succession;

mod chain;
pub use chain::Chain;

mod non_empty;
pub use non_empty::NonEmpty;

mod backing_utxo;
pub use backing_utxo::BackingUtxo;

use cardano::{BlockDepth, BlockHeight};

// Models the on-chain backing of L2 payments by UTXOs locked on the L1.
//
// A `Backing` is a set of `Chain`s. Each `Chain` is either `Live` (its tip
// is currently observed on-chain and can back payments) or `Lost` (its tip
// has vanished and it is retained only as a rollback fallback).
//
// Within a chain, lineage is tracked as a non-empty sequence of `BackingUtxo`s.
// Lineage can only be extended when a witnessed spend is observed (either via
// the chain indexer or via a server-submitted TX). In snapshot mode, chains
// are always singletons; lineage cannot be threaded across snapshot boundaries.
//
// Exposure is reported as a discretized step function over `DepthBucket`s,
// suitable for fee calculation without floating point.

// ---------------------------------------------------------------------------
// Depth buckets — coarse discretization of rollback risk
// ---------------------------------------------------------------------------

/// Coarse confirmation depth buckets. Finer granularity is unnecessary:
/// the meaningful risk distinctions are between these bands, not within them.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Encode, Decode,
)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
pub enum DepthBucket {
    /// Seen on-chain but very shallow — adversarially exploitable.
    #[n(0)]
    Unconfirmed,
    /// A few confirmations — rollback possible, elevated risk.
    #[n(1)]
    Shallow,
    /// Moderate confirmations — rollback unlikely but not negligible.
    #[n(2)]
    Probable,
    /// Deep enough to treat as practically final for fee purposes.
    #[n(3)]
    Deep,
    /// Beyond the finality window — floor is settled, zero exposure.
    #[n(4)]
    Settled,
}

impl DepthBucket {
    /// Classify a raw `BlockDepth` into a bucket.
    /// Thresholds are illustrative; adjust to your chain's finality model.
    pub fn from_depth(depth: BlockDepth) -> Self {
        match depth.0 {
            0..=2 => DepthBucket::Unconfirmed,
            3..=5 => DepthBucket::Shallow,
            6..=14 => DepthBucket::Probable,
            15..=29 => DepthBucket::Deep,
            _ => DepthBucket::Settled,
        }
    }
}

// ---------------------------------------------------------------------------
// Exposure — the risk step function
// ---------------------------------------------------------------------------

/// A single step in the coverage step function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageStep {
    /// Amount covered up to (and including) this step.
    pub cumulative_amount: u64,
    /// Rollback exposure for this slice, expressed as basis points (0–10_000).
    /// 0 = zero exposure (settled floor). 10_000 = fully unbacked.
    pub exposure_bps: u32,
    /// The depth bucket driving this exposure level.
    pub bucket: DepthBucket,
}

/// The answer to `exposureFor(Z)`.
#[derive(Debug, Clone)]
pub enum Exposure {
    /// Z ≤ settled floor across all live chains. Zero rollback exposure.
    FullyCovered,
    /// Z exceeds total live backing. No coverage available for the full amount.
    Unbacked,
    /// Partial coverage: some amount is settled, some is exposed.
    /// Steps are ordered from most-settled to least-settled.
    PartiallyExposed(Vec<CoverageStep>),
}

// ---------------------------------------------------------------------------
// Backing — the top-level type
// ---------------------------------------------------------------------------

pub enum BackingError {}

/// The complete set of backing chains for an L2 commitment.
/// Empty = no backing whatsoever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[repr(transparent)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Backing(#[n(0)] pub Vec<Chain>);

impl Backing {
    // --- Constructors -------------------------------------------------------

    /// No backing.
    pub fn empty() -> Self {
        Backing(vec![])
    }

    /// A single live chain from a freshly observed UTXO.
    pub fn new(utxo: BackingUtxo) -> Self {
        Backing(vec![Chain::new(utxo)])
    }

    /// Construct from an arbitrary set of chains (e.g. on indexer reconciliation).
    pub fn from_chains(chains: Vec<Chain>) -> Self {
        Backing(chains)
    }

    // --- Chain management ---------------------------------------------------

    pub fn append(&mut self, chains: Vec<Chain>) {
        let mut chains = chains;
        self.0.append(&mut chains);
    }

    pub fn push(&mut self, chain: Chain) {
        self.0.push(chain);
    }

    /// The assumptions of a snapshot:
    ///
    /// + All utxos are at tip.
    /// + Any new utxo is added as a singleton.
    /// + If a utxo matches an existing one then
    ///     + if its last, then continue.
    ///     + else split decendents into `Lost` chain.
    /// + Any existing chain with no matching utxo, is lost.
    ///
    /// The logic relies on the inability to split a chain.
    /// ie, there is at most one Utxo in a snapshot that correpsonds to a chain
    pub fn snapshot(
        &mut self,
        block_height: BlockHeight,
        utxos: Vec<BackingUtxo>,
    ) -> Result<(), BackingError> {
        let mut utxos = utxos;
        for chain in self.0.iter_mut() {
            for utxo in utxos.iter() {
                let _ = utxo;
                todo!()
            }
            chain.lose(block_height);
            todo!()
        }
        self.append(utxos.into_iter().map(Chain::new).collect());
        todo!()
    }

    /// The assumptions of a succession:
    ///
    /// + If the parent is `last`, then the child is appended.
    /// If `is_tip == true`, then chain is treated as `Chain::Live`.
    /// + If the parent already has child, then the exisitng lineage is split off into a
    /// `Chain::Lost`. and (new) child is appended to Parent.
    /// + If there is no recorded parent, a new singleton chain with the child,
    /// and return an error `NoParent`
    /// + If the child already exists and is already succeeding parent, then error already exists.
    /// + If the child is the head of an existing chain, the chains are merged.
    /// + Else the child is succeding not the parent, then something has gone very wrong.
    /// Error Inconsistent update
    pub fn succede(&mut self, succession: Succession, is_tip: bool) -> Result<(), BackingError> {
        let _ = (succession, is_tip);
        todo!()
    }

    pub fn rollback(&mut self, block_height: BlockHeight) {
        let _ = block_height;
        todo!()
    }
}
