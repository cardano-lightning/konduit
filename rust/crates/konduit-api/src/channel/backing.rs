pub mod cardano;

mod succession;
use minicbor::{Decode, Encode};
use serde::{Serialize, Deserialize};
pub use succession::Succession;

mod chain;
pub use chain::Chain;

mod non_empty;
pub use non_empty::NonEmpty;

mod backing_utxo;
pub use backing_utxo::BackingUtxo;

use crate::channel::backing::cardano::{BlockDepth, BlockHeight};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepthBucket {
    /// Seen on-chain but very shallow — adversarially exploitable.
    Unconfirmed,
    /// A few confirmations — rollback possible, elevated risk.
    Shallow,
    /// Moderate confirmations — rollback unlikely but not negligible.
    Probable,
    /// Deep enough to treat as practically final for fee purposes.
    Deep,
    /// Beyond the finality window — floor is settled, zero exposure.
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

pub enum BackingError {
}

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
        // FIXME :: How to append to vecs without faffing wiht muts
        let mut chains = chains;
        self.0.append(&mut chains);
    }

    pub fn push(&mut self, chain: Chain) {
        self.0.push(chain);
    }

    // /// Remove chains whose tips have been absent beyond the finality window
    // /// and whose predecessors are also settled — safe to discard.
    // pub fn gc_settled_lost(&mut self, finality_depth: BlockDepth) {
    //     self.0.retain(|chain| {
    //         match chain {
    //             Chain::Lost { links, .. } => {
    //                 // Retain if any link is not yet settled — still a fallback.
    //                 links.iter().any(|u| u.depth < finality_depth)
    //             }
    //             Chain::Live { .. } => true,
    //         }
    //     });
    // }

    // // --- Derived values -----------------------------------------------------

    // // /// All live chains.
    // // pub fn live_chains(&self) -> impl Iterator<Item = &Chain> {
    // //     self.0.iter().filter(|c| c.is_live())
    // // }

    // // /// The effective settled floor across all live chains.
    // // /// We take the max: a single well-settled chain is sufficient.
    // // pub fn settled_floor(&self) -> u64 {
    // //     self.live_chains()
    // //         .map(|c| c.settled_floor())
    // //         .max()
    // //         .unwrap_or(0)
    // // }

    // // /// The total live backing at tip (max across live chains).
    // // /// We take max, not sum: mimics are redundant, not additive.
    // // pub fn effective_amount(&self) -> u64 {
    // //     self.live_chains()
    // //         .map(|c| c.tip_amount())
    // //         .max()
    // //         .unwrap_or(0)
    // // }

    // /// The best (deepest) bucket across live chain tips.
    // pub fn best_bucket(&self) -> Option<DepthBucket> {
    //     self.live_chains().filter_map(|c| c.tip_bucket()).max()
    // }

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
                // Find a Utxo in snapshot;
                // If yes, pop
                todo!()
            }
            // Else, chain is lost
            chain.lose(block_height);
            todo!()
        }
        self.append(utxos.into_iter().map(|u| Chain::new(u)).collect());
        todo!()
    }

    /// The assumptions of a successsion:
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
        todo!()
    }

    pub fn rollback(&mut self, block_height: BlockHeight) {
        todo!()
    }

    // // --- Exposure -----------------------------------------------------------

    // /// Compute the discretized coverage step function for a payment of size `z`.
    // ///
    // /// Returns `FullyCovered` if `z` is within the settled floor.
    // /// Returns `Unbacked` if `z` exceeds all live backing.
    // /// Otherwise returns `PartiallyExposed` with steps describing each
    // /// coverage slice and its exposure in basis points.
    // ///
    // /// `bucket_exposure_bps` maps a `DepthBucket` to basis points of exposure
    // /// (0 = zero risk, 10_000 = certain loss). Supplied by the caller so that
    // /// `Backing` remains agnostic of the risk model.
    // pub fn exposure_for(
    //     &self,
    //     z: u64,
    //     bucket_exposure_bps: impl Fn(DepthBucket) -> u32,
    // ) -> Exposure {
    //     let floor = self.settled_floor();
    //     let total = self.effective_amount();

    //     if z <= floor {
    //         return Exposure::FullyCovered;
    //     }

    //     if z > total {
    //         return Exposure::Unbacked;
    //     }

    //     // Build the step function over the exposed slice (floor, z].
    //     // Collect all live tips, deduplicate by bucket (take deepest amount
    //     // per bucket), then emit steps from most-settled to least-settled.
    //     let mut steps: Vec<CoverageStep> = {
    //         // Group live chain tips by bucket, taking max amount per bucket.
    //         let mut by_bucket: std::collections::BTreeMap<DepthBucket, u64> =
    //             std::collections::BTreeMap::new();

    //         for chain in self.live_chains() {
    //             if let Some(bucket) = chain.tip_bucket() {
    //                 let amt = chain.tip_amount();
    //                 let entry = by_bucket.entry(bucket).or_insert(0);
    //                 if amt > *entry {
    //                     *entry = amt;
    //                 }
    //             }
    //         }

    //         // Emit steps: for each bucket (deepest first), the slice between
    //         // the previous cumulative amount and this bucket's amount.
    //         let mut prev = floor;
    //         let mut steps = vec![];

    //         // BTreeMap iterates in key order; DepthBucket derives Ord deepest-last,
    //         // so we reverse to go deepest-first (lowest exposure first).
    //         for (bucket, cumulative) in by_bucket.into_iter().rev() {
    //             if cumulative <= prev {
    //                 continue;
    //             }
    //             let slice_top = cumulative.min(z);
    //             steps.push(CoverageStep {
    //                 cumulative_amount: slice_top,
    //                 exposure_bps: bucket_exposure_bps(bucket),
    //                 bucket,
    //             });
    //             prev = slice_top;
    //             if prev >= z {
    //                 break;
    //             }
    //         }

    //         steps
    //     };

    //     // Ensure steps are ordered settled → unconfirmed (exposure ascending).
    //     steps.sort_by_key(|s| s.exposure_bps);

    //     Exposure::PartiallyExposed(steps)
    // }

    // /// Convenience: compute a single scalar fee in u64 given a risk
    // /// weight function (basis points per u64 at each bucket).
    // ///
    // /// fee = Σ slice_amount_i × exposure_bps_i / 10_000
    // pub fn fee_for(&self, z: u64, bucket_exposure_bps: impl Fn(DepthBucket) -> u32) -> u64 {
    //     match self.exposure_for(z, &bucket_exposure_bps) {
    //         Exposure::FullyCovered => 0,
    //         Exposure::Unbacked => u64::MAX, // caller should reject
    //         Exposure::PartiallyExposed(steps) => {
    //             let mut prev = self.settled_floor();
    //             let mut fee: u64 = 0;
    //             for step in &steps {
    //                 let slice = step.cumulative_amount.saturating_sub(prev);
    //                 fee = fee.saturating_add(
    //                     (slice as u128 * step.exposure_bps as u128 / 10_000) as u64,
    //                 );
    //                 prev = step.cumulative_amount;
    //             }
    //             fee
    //         }
    //     }
    // }
}

// // ---------------------------------------------------------------------------
// // Default risk model (illustrative)
// // ---------------------------------------------------------------------------
// 
// /// A simple default mapping from depth bucket to exposure basis points.
// /// Replace with your actual risk model.
// pub fn default_exposure_bps(bucket: DepthBucket) -> u32 {
//     match bucket {
//         DepthBucket::Settled => 0,
//         DepthBucket::Deep => 10,         // 0.1%
//         DepthBucket::Probable => 50,     // 0.5%
//         DepthBucket::Shallow => 200,     // 2%
//         DepthBucket::Unconfirmed => 800, // 8%
//     }
// }
// 
// // ---------------------------------------------------------------------------
// // Tests
// // ---------------------------------------------------------------------------
// 
// #[cfg(test)]
// mod tests {
//     use super::*;
// 
//     fn utxo(n: u8) -> OutputReference {
//         OutputReference {
//             transaction_id: [n; 32],
//             output_index: 0,
//         }
//     }
// 
//     fn backing_utxo(n: u8, amount: u64, depth: u32) -> BackingUtxo {
//         BackingUtxo::new(utxo(n), amount, BlockDepth(depth))
//     }
// 
//     #[test]
//     fn empty_backing_is_unbacked() {
//         let b = Backing::empty();
//         assert!(matches!(
//             b.exposure_for(1000, default_exposure_bps),
//             Exposure::Unbacked
//         ));
//     }
// 
//     #[test]
//     fn settled_utxo_gives_full_coverage() {
//         // depth 100 >> finality threshold → Settled bucket
//         let b = Backing::new(backing_utxo(1, 1_000_000, 100));
//         assert!(matches!(
//             b.exposure_for(500_000, default_exposure_bps),
//             Exposure::FullyCovered
//         ));
//     }
// 
//     #[test]
//     fn shallow_utxo_gives_partial_exposure() {
//         let b = Backing::new(backing_utxo(1, 1_000_000, 4)); // Shallow
//         let exp = b.exposure_for(500_000, default_exposure_bps);
//         assert!(matches!(exp, Exposure::PartiallyExposed(_)));
//     }
// 
//     #[test]
//     fn exceeding_backing_is_unbacked() {
//         let b = Backing::new(backing_utxo(1, 1_000_000, 4));
//         assert!(matches!(
//             b.exposure_for(2_000_000, default_exposure_bps),
//             Exposure::Unbacked
//         ));
//     }
// 
//     #[test]
//     fn lost_chain_does_not_contribute() {
//         let chain = Chain::new(backing_utxo(1, 1_000_000, 100)).lose();
//         let b = Backing::from_chains(vec![chain]);
//         assert!(matches!(
//             b.exposure_for(1, default_exposure_bps),
//             Exposure::Unbacked
//         ));
//     }
// 
//     #[test]
//     fn mimic_does_not_aggregate() {
//         // Two identical live chains — effective amount is max, not sum.
//         let b = Backing::from_chains(vec![
//             Chain::new(backing_utxo(1, 1_000_000, 4)),
//             Chain::new(backing_utxo(2, 1_000_000, 4)),
//         ]);
//         assert_eq!(b.effective_amount(), (1_000_000));
//     }
// 
//     #[test]
//     fn chain_extend_requires_witness() {
//         let tip = backing_utxo(1, 1_000_000, 4);
//         let successor = backing_utxo(2, 1_000_000, 1);
//         let chain = Chain::new(tip.clone());
// 
//         let bad_witness = LineageWitness {
//             spent: utxo(99), // wrong
//             successor: utxo(2),
//         };
//         let result = chain.extend(successor, &bad_witness);
//         assert!(matches!(result, Err(ChainError::WitnessMismatch)));
//     }
// 
//     #[test]
//     fn fee_is_zero_for_fully_covered() {
//         let b = Backing::new(backing_utxo(1, 1_000_000, 100));
//         assert_eq!(b.fee_for(500_000, default_exposure_bps), 0);
//     }
// }
// 
// pub enum BackingError {
//     NoParent,
// }
