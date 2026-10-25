use std::cmp;

use cardano_sdk::{Milliseconds, PosixMilliseconds, Slot};

/// The validity window of a transaction, in POSIX time.
///
/// The bounds are *timestamps*, not durations: they are compared against the
/// validity range the ledger hands to a Plutus script, and against the
/// timestamps stored in a channel's datum. They are anchored at the chain's
/// current tip — see [`Bounds::around_tip`] — so that they live in the
/// chain's clock rather than in the wallet's wall clock.
#[derive(Debug, Clone, Default)]
pub struct Bounds {
    pub lower: Option<PosixMilliseconds>,
    pub upper: Option<PosixMilliseconds>,
}

impl Bounds {
    /// How many slots a transaction's validity window spans on either side
    /// of the chain's tip.
    ///
    /// The window has to be wide enough for the transaction to be *included*:
    /// it is built (the Plutus scripts are evaluated locally, which takes a
    /// second or two), submitted, and only then does the chain pick it up —
    /// and a chain does not produce a block every slot. On the local testnet
    /// (200ms slots) blocks arrive every 1–6 seconds, so 30 slots (6 seconds)
    /// was not enough: the transaction was accepted by the node but never
    /// made it into a block, and the channel it was supposed to close stayed
    /// open.
    ///
    /// It also has to stay within what the node is willing to schedule: a
    /// window that reaches too far ahead is rejected outright with
    /// `EraNotPastHorizon` ("SlotNo … is not in the past horizon of the era
    /// …"), which happens well before 1000 slots on the testnet.
    ///
    /// 300 slots sits between the two: 60 seconds on the testnet (ten times
    /// the worst block delay we have seen), five minutes on a chain with
    /// one-second slots.
    pub const MARGIN_SLOTS: u64 = 300;

    pub fn lower(lower: PosixMilliseconds) -> Self {
        Self {
            lower: Some(lower),
            upper: None,
        }
    }

    pub fn upper(upper: PosixMilliseconds) -> Self {
        Self {
            lower: None,
            upper: Some(upper),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let lower = match (self.lower, other.lower) {
            (Some(a), Some(b)) => Some(cmp::max(a, b)),
            (a, b) => a.or(b),
        };
        let upper = match (self.upper, other.upper) {
            (Some(a), Some(b)) => Some(cmp::min(a, b)),
            (a, b) => a.or(b),
        };

        Self { lower, upper }
    }

    /// A validity window around the chain's current tip.
    ///
    /// The margin is expressed in *slots*, not in time: a chain with 200ms
    /// slots and one with 20s slots then get the same number of slots to
    /// include the transaction in, while a window expressed in minutes would
    /// be 5400 slots ahead on a fast chain — too far ahead for the node to
    /// schedule.
    pub fn around_tip(
        tip_time: PosixMilliseconds,
        slot_length_ms: Milliseconds,
        margin_slots: Slot,
    ) -> Self {
        let margin = slot_length_ms * u64::from(margin_slots);
        Bounds {
            lower: Some(tip_time - margin),
            upper: Some(tip_time + margin),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn around_tip_spans_the_margin_on_both_sides() {
        let tip = PosixMilliseconds::new(1_000_000);
        let margin = Milliseconds::new(200 * Bounds::MARGIN_SLOTS);
        let bounds =
            Bounds::around_tip(tip, Milliseconds::new(200), Slot::new(Bounds::MARGIN_SLOTS));

        assert_eq!(bounds.lower, Some(tip - margin));
        assert_eq!(bounds.upper, Some(tip + margin));
    }

    #[test]
    fn the_margin_covers_a_chain_that_does_not_produce_a_block_every_slot() {
        // 300 slots of a 200ms chain is a minute — ten times the worst block
        // delay we have seen on the local testnet. A margin that only spans a
        // couple of slots lets a transaction expire before it is included.
        assert!(Bounds::MARGIN_SLOTS * 200 >= 60_000);
    }
}
