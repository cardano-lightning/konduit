//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Units of time and of chain position.
//!
//! Cardano measures the same instants three different ways: in *slots* (the
//! chain's own clock), in *milliseconds* (what the ledger hands to Plutus
//! scripts) and in *seconds* (what a wallet's wall clock says). Mixing them up
//! is not a cosmetic problem: a slot number computed with the wrong slot
//! length is a transaction that is expired before it is submitted.
//!
//! FIXME :: Promote these types to `konduit-data` (project level) once every
//! crate agrees on one set of time units. The indexer carries its own
//! `SlotNo` for now, and `konduit-data` has a `Duration` that is serialised as
//! milliseconds.

use crate::cbor;
use std::{fmt, ops};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A slot number — the chain's unit of time.
///
/// FIXME :: Promote to `konduit-data` (project level), see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, cbor::Encode, cbor::Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Slot(#[n(0)] u64);

impl Slot {
    pub fn new(slot: u64) -> Self {
        Self(slot)
    }

    /// The number of slots between `self` and `other` (zero when `other` is
    /// earlier).
    pub fn distance_to(self, other: Self) -> u64 {
        other.0.saturating_sub(self.0)
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slot {}", self.0)
    }
}

impl From<u64> for Slot {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Slot> for u64 {
    fn from(value: Slot) -> Self {
        value.0
    }
}

/// A count of milliseconds.
///
/// FIXME :: Promote to `konduit-data` (project level), see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, cbor::Encode, cbor::Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Milliseconds(#[n(0)] u64);

impl Milliseconds {
    pub fn new(milliseconds: u64) -> Self {
        Self(milliseconds)
    }

    pub fn as_millis(self) -> u64 {
        self.0
    }

    /// The same duration expressed in seconds (rounded down).
    pub fn as_secs(self) -> u64 {
        self.0 / 1_000
    }
}

impl fmt::Display for Milliseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

impl From<u64> for Milliseconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Milliseconds> for u64 {
    fn from(value: Milliseconds) -> Self {
        value.0
    }
}

impl ops::Mul<u64> for Milliseconds {
    type Output = Milliseconds;

    fn mul(self, rhs: u64) -> Self::Output {
        Milliseconds(self.0.saturating_mul(rhs))
    }
}

/// A POSIX timestamp: milliseconds since the Unix epoch.
///
/// This is the unit the ledger uses when it hands a transaction's validity
/// range to a Plutus script, and the unit in which a cheque's timeout is
/// stored on chain.
///
/// FIXME :: Promote to `konduit-data` (project level), see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, cbor::Encode, cbor::Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(transparent)]
#[cbor(transparent)]
pub struct PosixMilliseconds(#[n(0)] u64);

impl PosixMilliseconds {
    pub fn new(milliseconds: u64) -> Self {
        Self(milliseconds)
    }

    /// The current time, in POSIX milliseconds.
    ///
    /// This is the local clock — the machine's idea of "now", not the chain's.
    /// `web-time` is used so that the browser gets a real clock too: `std`'s
    /// `SystemTime::now` panics on `wasm32-unknown-unknown`.
    pub fn now() -> Self {
        let millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default();
        PosixMilliseconds::new(millis)
    }

    pub fn as_millis(self) -> u64 {
        self.0
    }

    /// The same instant expressed in seconds since the epoch (rounded down).
    pub fn as_secs(self) -> u64 {
        self.0 / 1_000
    }
}

impl fmt::Display for PosixMilliseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

impl From<u64> for PosixMilliseconds {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<PosixMilliseconds> for u64 {
    fn from(value: PosixMilliseconds) -> Self {
        value.0
    }
}

impl ops::Add<Milliseconds> for PosixMilliseconds {
    type Output = PosixMilliseconds;

    fn add(self, rhs: Milliseconds) -> Self::Output {
        PosixMilliseconds(self.0.saturating_add(rhs.as_millis()))
    }
}

impl ops::Sub<Milliseconds> for PosixMilliseconds {
    type Output = PosixMilliseconds;

    fn sub(self, rhs: Milliseconds) -> Self::Output {
        PosixMilliseconds(self.0.saturating_sub(rhs.as_millis()))
    }
}

/// The chain's slot ⇄ time mapping, anchored at the Shelley era.
///
/// The anchor is the first slot of the Shelley era and the POSIX time it sits
/// at: the mapping counts from there, in slots of the chain's slot length.
/// The Byron slots that precede it are outside the mapping — a validity range
/// starts no earlier than the Shelley anchor.
///
/// This is the ledger's own mapping, the one it uses to turn a transaction's
/// validity range into slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShelleySlotConfig {
    /// The POSIX time of [`Self::first_era_slot`].
    pub start_time: PosixMilliseconds,
    /// The first slot of the Shelley era — the anchor of the mapping.
    pub first_era_slot: Slot,
    /// The length of a slot.
    pub slot_length_ms: Milliseconds,
}

impl ShelleySlotConfig {
    /// The slot a POSIX time falls into.
    ///
    /// The time is counted from the anchor, in whole slots.
    pub fn posix_to_slot(&self, posix: PosixMilliseconds) -> Slot {
        let delta_ms = posix
            .as_millis()
            .saturating_sub(self.start_time.as_millis());
        let slots = delta_ms / self.slot_length_ms.as_millis().max(1);
        Slot::new(u64::from(self.first_era_slot) + slots)
    }

    /// The POSIX time of a slot.
    ///
    /// The inverse of [`Self::posix_to_slot`].
    pub fn slot_to_posix(&self, slot: Slot) -> PosixMilliseconds {
        let slots = self.first_era_slot.distance_to(slot);
        self.start_time + self.slot_length_ms * slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliseconds_multiply_by_a_count() {
        assert_eq!(Milliseconds::new(200) * 30, Milliseconds::new(6_000));
    }

    #[test]
    fn posix_time_is_shifted_by_milliseconds() {
        let time = PosixMilliseconds::new(1_000);
        assert_eq!(time + Milliseconds::new(500), PosixMilliseconds::new(1_500));
        assert_eq!(time - Milliseconds::new(500), PosixMilliseconds::new(500));
    }

    #[test]
    fn the_slot_config_counts_from_its_anchor() {
        let config = ShelleySlotConfig {
            start_time: PosixMilliseconds::new(1_000),
            first_era_slot: Slot::new(10),
            slot_length_ms: Milliseconds::new(200),
        };

        // The anchor's own time is the anchor's slot, and a slot lasts until
        // the next one starts.
        assert_eq!(
            config.posix_to_slot(PosixMilliseconds::new(1_000)),
            Slot::new(10)
        );
        assert_eq!(
            config.posix_to_slot(PosixMilliseconds::new(1_199)),
            Slot::new(10)
        );
        assert_eq!(
            config.posix_to_slot(PosixMilliseconds::new(1_200)),
            Slot::new(11)
        );
        assert_eq!(
            config.slot_to_posix(Slot::new(11)),
            PosixMilliseconds::new(1_200)
        );
    }

    #[test]
    fn slot_distance_is_saturating() {
        assert_eq!(Slot::new(10).distance_to(Slot::new(15)), 5);
        assert_eq!(Slot::new(15).distance_to(Slot::new(10)), 0);
    }

    #[test]
    fn now_is_after_the_epoch() {
        assert!(PosixMilliseconds::now().as_millis() > 1_000_000_000_000);
    }
}
