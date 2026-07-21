//! Behavior every `Backend` impl must satisfy, backend-agnostic.
//! Each backend's own `tests` module constructs an instance (with a
//! `FakeClock` where `at` needs to be controlled) and calls these.

#![cfg(test)]

use std::cell::Cell;
use std::rc::Rc;

use konduit_data::{Duration, Lock, Tag};

use super::{Backend, Error};

/// Controllable clock for deterministic pruning tests. `.set(...)`
/// between `commit` calls to control the `at` each one records.
/// Not `Send` — fine, since `Backend` itself is `?Send`.
#[derive(Clone)]
pub struct FakeClock(Rc<Cell<Duration>>);

impl FakeClock {
    pub fn at(initial: Duration) -> Self {
        Self(Rc::new(Cell::new(initial)))
    }

    pub fn set(&self, at: Duration) {
        self.0.set(at);
    }

    pub fn as_fn(&self) -> impl Fn() -> Duration + 'static {
        let cell = self.0.clone();
        move || cell.get()
    }
}

pub async fn fresh_insert_succeeds(backend: &dyn Backend) {
    let lock = Lock::from([0; 32]);
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();
    let stored = backend.get(&lock).await.unwrap().unwrap();
    assert_eq!(stored.tag(), &Tag::from(vec![1]));
    assert_eq!(stored.index(), 0);
}

pub async fn conflicting_target_is_rejected(backend: &dyn Backend) {
    let lock = Lock::from([2; 32]);
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();
    let err = backend
        .commit(lock.clone(), Tag::from(vec![2]), 0)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict));
    let stored = backend.get(&lock).await.unwrap().unwrap();
    assert_eq!(stored.tag(), &Tag::from(vec![1])); // untouched
}

pub async fn sweep_on_empty_store_is_noop(backend: &dyn Backend) {
    assert_eq!(
        backend
            .sweep_before(Duration::from_millis(100))
            .await
            .unwrap(),
        0
    );
}

pub async fn sweep_removes_only_old_entries(backend: &dyn Backend, clock: &FakeClock) {
    let old_lock = Lock::from([3; 32]);
    let new_lock = Lock::from([4; 32]);

    clock.set(Duration::from_millis(50));
    backend
        .commit(old_lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();

    clock.set(Duration::from_millis(500));
    backend
        .commit(new_lock.clone(), Tag::from(vec![2]), 0)
        .await
        .unwrap();

    let removed = backend
        .sweep_before(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert!(backend.get(&old_lock).await.unwrap().is_none());
    assert!(backend.get(&new_lock).await.unwrap().is_some());
}

pub async fn sweep_boundary_is_inclusive(backend: &dyn Backend, clock: &FakeClock) {
    let lock = Lock::from([5; 32]);
    clock.set(Duration::from_millis(100));
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();

    let removed = backend
        .sweep_before(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert!(backend.get(&lock).await.unwrap().is_none());
}

pub async fn same_target_recommit_is_noop(backend: &dyn Backend, clock: &FakeClock) {
    let lock = Lock::from([1; 32]);
    clock.set(Duration::from_millis(100));
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();

    clock.set(Duration::from_millis(200));
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap(); // no-op

    let stored = backend.get(&lock).await.unwrap().unwrap();
    assert_eq!(*stored.at(), Duration::from_millis(100)); // untouched
}

pub async fn recommit_does_not_refresh_at(backend: &dyn Backend, clock: &FakeClock) {
    let lock = Lock::from([6; 32]);
    clock.set(Duration::from_millis(50));
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap();

    clock.set(Duration::from_millis(500));
    backend
        .commit(lock.clone(), Tag::from(vec![1]), 0)
        .await
        .unwrap(); // ignored

    let removed = backend
        .sweep_before(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(removed, 1); // swept on original at=50, not the discarded at=500
    assert!(backend.get(&lock).await.unwrap().is_none());
}
