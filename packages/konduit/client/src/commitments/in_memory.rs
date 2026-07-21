//! in_memory.rs — Reference in-memory `Backend`, for tests.
//!
//! Correct, not fast — one global mutex around a plain map. The mutex
//! guard holds check-then-write as one critical section, so concurrent
//! `commit`s on the same lock can't race.
//!
//! `system_now()` uses `std::time::SystemTime`, which panics at *call*
//! time (not compile time) on `wasm32-unknown-unknown` — fine as long
//! as this backend is only actually used natively; `IdbBackend` is the
//! wasm-side implementation.

use std::collections::BTreeMap;
use std::sync::Mutex;

use konduit_data::{Duration, Lock, Tag};

use super::{Backend, Commitment, Error};

pub struct InMemory {
    entries: Mutex<BTreeMap<Lock, Commitment>>,
    now: Box<dyn Fn() -> Duration>,
}

impl InMemory {
    pub fn new() -> Self {
        Self::with_clock(|| crate::time::now().unwrap())
    }

    pub fn with_clock(now: impl Fn() -> Duration + 'static) -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            now: Box::new(now),
        }
    }
}

impl Default for InMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait(?Send)]
impl Backend for InMemory {
    async fn commit(&self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        match entries.entry(lock) {
            std::collections::btree_map::Entry::Occupied(slot) => {
                let prior = slot.get();
                if prior.tag() == &tag && prior.index() == index {
                    Ok(())
                } else {
                    Err(Error::Conflict)
                }
            }
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(Commitment::new(tag, index, (self.now)()));
                Ok(())
            }
        }
    }

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        Ok(entries.get(lock).cloned())
    }

    async fn sweep_before(&self, threshold: Duration) -> Result<u64, Error> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        let before = entries.len();
        entries.retain(|_, commitment| *commitment.at() > threshold);
        Ok((before - entries.len()) as u64)
    }
}

#[cfg(test)]
mod tests {
    use konduit_data::Duration;

    use super::InMemory;
    use crate::commitments::backend_test_suite as suite;

    fn temp_backend(clock: impl Fn() -> Duration + 'static) -> InMemory {
        InMemory::with_clock(clock)
    }

    #[tokio::test]
    async fn fresh_insert_succeeds() {
        suite::fresh_insert_succeeds(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn conflicting_target_is_rejected() {
        suite::conflicting_target_is_rejected(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn sweep_on_empty_store_is_noop() {
        suite::sweep_on_empty_store_is_noop(&temp_backend(|| Duration::from_millis(0))).await;
    }

    #[tokio::test]
    async fn sweep_removes_only_old_entries() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::sweep_removes_only_old_entries(&backend, &clock).await;
    }

    #[tokio::test]
    async fn sweep_boundary_is_inclusive() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::sweep_boundary_is_inclusive(&backend, &clock).await;
    }

    #[tokio::test]
    async fn same_target_recommit_is_noop() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::same_target_recommit_is_noop(&backend, &clock).await;
    }

    #[tokio::test]
    async fn recommit_does_not_refresh_at() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn());
        suite::recommit_does_not_refresh_at(&backend, &clock).await;
    }
}
