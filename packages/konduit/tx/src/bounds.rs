use std::cmp;

use cardano_sdk::{PosixMilliseconds, ShelleySlotConfig, Slot};

/// The validity window of a transaction, in POSIX time.
///
/// The bounds are *timestamps*, not durations: they are compared against the
/// validity range the ledger hands to a Plutus script, and against the
/// timestamps stored in a channel's datum. They are anchored at the current
/// time — see [`Bounds::around_now`].
#[derive(Debug, Clone, Default)]
pub struct Bounds {
    pub lower: Option<PosixMilliseconds>,
    pub upper: Option<PosixMilliseconds>,
}

impl Bounds {
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

    /// A validity window around an explicit instant.
    ///
    /// The bounds are the starts of the slots `margin_slots` slots away from
    /// the instant's slot: the ledger maps the validity range back to slots,
    /// so bounds on slot boundaries give the window we asked for. The margin
    /// is counted in slots, not in time, so a fast chain does not get a window
    /// thousands of slots ahead — too far ahead for the node to schedule.
    pub fn around(now: PosixMilliseconds, config: &ShelleySlotConfig, margin_slots: Slot) -> Self {
        let slot = u64::from(config.posix_to_slot(now));
        let margin = u64::from(margin_slots);
        Bounds {
            lower: Some(config.slot_to_posix(Slot::new(slot.saturating_sub(margin)))),
            upper: Some(config.slot_to_posix(Slot::new(slot.saturating_add(margin)))),
        }
    }

    /// A validity window around the current time.
    ///
    /// "Now" is the local clock, which only approximates the chain's time.
    pub fn around_now(config: &ShelleySlotConfig, margin_slots: Slot) -> Self {
        Self::around(PosixMilliseconds::now(), config, margin_slots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_spans_the_margin_on_both_sides() {
        // One second slots anchored at the epoch, so a millisecond timestamp
        // maps to the slot of the same number.
        let config = ShelleySlotConfig {
            start_time: PosixMilliseconds::new(0),
            first_era_slot: Slot::new(0),
            slot_length_ms: cardano_sdk::Milliseconds::new(1_000),
        };
        let now = PosixMilliseconds::new(1_000_000);
        let margin = Bounds::MARGIN_SLOTS;

        let bounds = Bounds::around(now, &config, Slot::new(margin));

        assert_eq!(
            config.posix_to_slot(bounds.lower.expect("the window has a lower bound")),
            Slot::new(1_000 - margin)
        );
        assert_eq!(
            config.posix_to_slot(bounds.upper.expect("the window has an upper bound")),
            Slot::new(1_000 + margin)
        );
    }

    #[test]
    fn the_window_is_on_the_slot_grid() {
        let config = ShelleySlotConfig {
            start_time: PosixMilliseconds::new(0),
            first_era_slot: Slot::new(0),
            slot_length_ms: cardano_sdk::Milliseconds::new(1_000),
        };
        let now = PosixMilliseconds::new(1_000_000);

        let bounds = Bounds::around(now, &config, Slot::new(Bounds::MARGIN_SLOTS));

        for bound in [bounds.lower, bounds.upper].into_iter().flatten() {
            let slot = config.posix_to_slot(bound);
            assert_eq!(config.slot_to_posix(slot), bound);
        }
    }

    #[test]
    fn the_window_around_now_contains_the_current_time() {
        let config = ShelleySlotConfig {
            start_time: PosixMilliseconds::new(0),
            first_era_slot: Slot::new(0),
            slot_length_ms: cardano_sdk::Milliseconds::new(1_000),
        };
        let bounds = Bounds::around_now(&config, Slot::new(Bounds::MARGIN_SLOTS));
        let now = PosixMilliseconds::now();

        assert!(bounds.lower.expect("the window has a lower bound") <= now);
        assert!(bounds.upper.expect("the window has an upper bound") >= now);
    }

    #[test]
    fn the_margin_covers_a_chain_that_does_not_produce_a_block_every_slot() {
        // 300 slots of a 200ms chain is a minute — ten times the worst block
        // delay we have seen on the local testnet. A margin that only spans a
        // couple of slots lets a transaction expire before it is included.
        let slot_length_ms: u64 = 200;
        assert!(slot_length_ms * Bounds::MARGIN_SLOTS >= 60_000);
    }
}
