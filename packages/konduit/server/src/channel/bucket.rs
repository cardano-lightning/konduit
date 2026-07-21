use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Bucket {
    #[n(0)]
    capacity: u64,
    #[n(1)]
    refill_rate: u64,
    #[n(2)]
    fill: u64,
    #[n(3)]
    last_update: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Overdrawn: Want {requested}, have {available}")]
    Overdrawn { requested: u64, available: u64 },
}

impl Bucket {
    /// Create a new bucket. Starts empty (full spending ability).
    ///
    /// - `capacity`: maximum units the bucket can accumulate
    /// - `refill_rate`: units shed per second (must be > 0)
    /// - `now`: current time in seconds
    ///
    /// # Panics
    /// Panics if `refill_rate` is zero.
    pub fn new(capacity: u64, refill_rate: u64, now: u64) -> Self {
        assert!(refill_rate > 0, "refill_rate must be greater than zero");
        Self {
            capacity,
            refill_rate,
            fill: 0,
            last_update: now,
        }
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn refill_rate(&self) -> u64 {
        self.refill_rate
    }

    pub fn last_update(&self) -> u64 {
        self.last_update
    }

    /// Remaining headroom at time `now`.
    pub fn available(&self, now: u64) -> u64 {
        self.capacity - self.current_fill(now)
    }

    /// Check whether consuming `amount` at time `now` would succeed, without
    /// mutating state.
    pub fn try_consume(&self, amount: u64, now: u64) -> Result<(), Error> {
        let available = self.available(now);
        if amount > available {
            return Err(Error::Overdrawn {
                requested: amount,
                available,
            });
        }
        Ok(())
    }

    /// Consume `amount` units at time `now`. Fails if it would exceed capacity.
    ///
    /// # Panics (debug only)
    /// Panics in debug builds if `now` is earlier than `last_update`.
    pub fn consume(&mut self, amount: u64, now: u64) -> Result<(), Error> {
        debug_assert!(now >= self.last_update, "now must not go backwards");
        let fill = self.current_fill(now);
        let available = self.capacity - fill;
        if amount > available {
            return Err(Error::Overdrawn {
                requested: amount,
                available,
            });
        }
        self.fill = fill + amount;
        self.last_update = now;
        Ok(())
    }

    fn current_fill(&self, now: u64) -> u64 {
        let elapsed = now.saturating_sub(self.last_update);
        self.fill
            .saturating_sub(self.refill_rate.saturating_mul(elapsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        let b = Bucket::new(100, 10, 0);
        assert_eq!(b.available(0), 100);
    }

    #[test]
    fn consume_increases_fill() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(30, 0).unwrap();
        assert_eq!(b.available(0), 70);
    }

    #[test]
    fn fill_decays_over_time() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(50, 0).unwrap();
        // After 2s: 50 - 20 = 30 fill, 70 available
        assert_eq!(b.available(2), 70);
    }

    #[test]
    fn fill_floors_at_zero() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(10, 0).unwrap();
        // After 5s: fully decayed, full headroom restored
        assert_eq!(b.available(5), 100);
    }

    #[test]
    fn consume_at_exact_capacity_succeeds() {
        let mut b = Bucket::new(100, 10, 0);
        assert!(b.consume(100, 0).is_ok());
        assert_eq!(b.available(0), 0);
    }

    #[test]
    fn consume_fails_when_over_capacity() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(90, 0).unwrap();
        assert_eq!(
            b.consume(20, 0),
            Err(Error::Overdrawn {
                requested: 20,
                available: 10
            })
        );
    }

    #[test]
    fn consume_succeeds_after_decay() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(100, 0).unwrap();
        // After 5s: 50 decayed, 50 available
        b.consume(50, 5).unwrap();
        assert_eq!(b.available(5), 0);
    }

    #[test]
    fn try_consume_does_not_mutate() {
        let b = Bucket::new(100, 10, 0);
        assert!(b.try_consume(50, 0).is_ok());
        assert_eq!(b.available(0), 100); // unchanged
    }

    #[test]
    fn try_consume_fails_when_over_capacity() {
        let mut b = Bucket::new(100, 10, 0);
        b.consume(90, 0).unwrap();
        assert_eq!(
            b.try_consume(20, 0),
            Err(Error::Overdrawn {
                requested: 20,
                available: 10
            })
        );
    }

    #[test]
    fn clock_going_backwards_saturates_to_zero_elapsed() {
        let mut b = Bucket::new(100, 10, 10);
        b.consume(50, 10).unwrap();
        // now=5 is before last_update=10; saturates to zero elapsed, fill unchanged
        assert_eq!(b.available(5), 50);
    }

    #[test]
    fn saturating_mul_prevents_overflow() {
        let b = Bucket::new(100, u64::MAX, 0);
        b.try_consume(50, 0).unwrap();
        // Large refill_rate * elapsed should not panic
        assert_eq!(b.available(u64::MAX), 100);
    }

    #[test]
    #[should_panic(expected = "refill_rate must be greater than zero")]
    fn zero_refill_rate_panics() {
        Bucket::new(100, 0, 0);
    }

    #[test]
    fn accessors_return_config() {
        let b = Bucket::new(200, 20, 42);
        assert_eq!(b.capacity(), 200);
        assert_eq!(b.refill_rate(), 20);
        assert_eq!(b.last_update(), 42);
    }
}
