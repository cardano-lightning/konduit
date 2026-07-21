#![cfg(test)]

use std::sync::atomic::{AtomicU32, Ordering};

use konduit_data::{Duration, Lock, Tag};
use wasm_bindgen_test::*;

use super::*;

wasm_bindgen_test_configure!(run_in_service_worker);

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// A fresh IndexedDB database name per test, so tests never see each
/// other's leftover state within the same test binary/session.
fn test_db_name() -> String {
    let n = TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("konduit-commitments-test-{n}")
}

fn commitment(tag: u8, index: u64, at: u64) -> Commitment {
    Commitment::new(Tag::from(vec![tag]), index, Duration::from_millis(at))
}

#[wasm_bindgen_test]
async fn fresh_insert_succeeds() {
    let backend = idb_backend::IdbBackend::open(&test_db_name())
        .await
        .unwrap();
    let lock = Lock::from([0; 32]);
    backend
        .upsert(lock.clone(), commitment(1, 0, 100))
        .await
        .unwrap();
    let stored = backend.get(&lock).await.unwrap();
    assert_eq!(stored, Some(commitment(1, 0, 100)));
}

#[wasm_bindgen_test]
async fn same_target_reupsert_refreshes_at() {
    let backend = idb_backend::IdbBackend::open(&test_db_name())
        .await
        .unwrap();
    let lock = Lock::from([1; 32]);
    backend
        .upsert(lock.clone(), commitment(1, 0, 100))
        .await
        .unwrap();
    backend
        .upsert(lock.clone(), commitment(1, 0, 200))
        .await
        .unwrap();
    let stored = backend.get(&lock).await.unwrap();
    assert_eq!(stored, Some(commitment(1, 0, 200)));
}

#[wasm_bindgen_test]
async fn conflicting_target_is_rejected() {
    let backend = idb_backend::IdbBackend::open(&test_db_name())
        .await
        .unwrap();
    let lock = Lock::from([2; 32]);
    backend
        .upsert(lock.clone(), commitment(1, 0, 100))
        .await
        .unwrap();
    let err = backend
        .upsert(lock.clone(), commitment(2, 0, 100))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Conflict));
    let stored = backend.get(&lock).await.unwrap();
    assert_eq!(stored, Some(commitment(1, 0, 100)));
}

#[wasm_bindgen_test]
async fn sweep_removes_only_old_entries() {
    let backend = idb_backend::IdbBackend::open(&test_db_name())
        .await
        .unwrap();
    let old_lock = Lock::from([3; 32]);
    let new_lock = Lock::from([4; 32]);
    backend
        .upsert(old_lock.clone(), commitment(1, 0, 50))
        .await
        .unwrap();
    backend
        .upsert(new_lock.clone(), commitment(2, 0, 500))
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
