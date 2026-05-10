use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Limit {
    #[n(0)]
    capacity: u64,
    #[n(1)]
    decay: u64,
    #[n(2)]
    bits: u64,
    #[n(3)]
    last_update: u64,
}

#[derive(Debug, PartialEq)]
pub struct Insufficient {
    pub requested: u64,
    pub available: u64,
}

impl Limit {
    /// Create a new limit. Starts at zero (full spending ability).
    /// `decay` is bits shed per second. `now` is in seconds.
    pub fn new(capacity: u64, decay: u64, now: u64) -> Self {
        Self {
            capacity,
            decay,
            bits: 0,
            last_update: now,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
    pub fn decay(&self) -> u64 {
        self.decay
    }
    pub fn bits(&self) -> u64 {
        self.bits
    }
    pub fn last_update(&self) -> u64 {
        self.last_update
    }

    /// Remaining headroom at time `now`.
    pub fn available(&self, now: u64) -> u64 {
        let elapsed = now.saturating_sub(self.last_update);
        let current = self.bits.saturating_sub(self.decay * elapsed);
        self.capacity - current
    }

    /// Try to spend `amount` bits. Fails if it would exceed capacity.
    pub fn spend(&mut self, amount: u64, now: u64) -> Result<(), Insufficient> {
        let elapsed = now.saturating_sub(self.last_update);
        self.bits = self.bits.saturating_sub(self.decay * elapsed);
        self.last_update = now;

        let available = self.capacity - self.bits;
        if amount > available {
            return Err(Insufficient {
                requested: amount,
                available,
            });
        }

        self.bits += amount;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let l = Limit::new(100, 10, 0);
        assert_eq!(l.bits(), 0);
        assert_eq!(l.available(0), 100);
    }

    #[test]
    fn spend_increases_bits() {
        let mut l = Limit::new(100, 10, 0);
        l.spend(30, 0).unwrap();
        assert_eq!(l.bits(), 30);
        assert_eq!(l.available(0), 70);
    }

    #[test]
    fn bits_decay_over_time() {
        let mut l = Limit::new(100, 10, 0);
        l.spend(50, 0).unwrap();
        // After 2s: 50 - 20 = 30 used, 70 available
        assert_eq!(l.available(2), 70);
    }

    #[test]
    fn bits_floor_at_zero() {
        let mut l = Limit::new(100, 10, 0);
        l.spend(10, 0).unwrap();
        // After 5s: fully decayed, full headroom restored
        assert_eq!(l.available(5), 100);
    }

    #[test]
    fn spend_fails_when_over_capacity() {
        let mut l = Limit::new(100, 10, 0);
        l.spend(90, 0).unwrap();
        assert_eq!(
            l.spend(20, 0),
            Err(Insufficient {
                requested: 20,
                available: 10
            })
        );
    }

    #[test]
    fn spend_succeeds_after_decay() {
        let mut l = Limit::new(100, 10, 0);
        l.spend(100, 0).unwrap();
        // After 5s: 50 decayed, 50 available
        l.spend(50, 5).unwrap();
        assert_eq!(l.available(5), 0);
    }

    #[test]
    fn accessors_return_config() {
        let l = Limit::new(200, 20, 42);
        assert_eq!(l.capacity(), 200);
        assert_eq!(l.decay(), 20);
        assert_eq!(l.last_update(), 42);
    }
}
