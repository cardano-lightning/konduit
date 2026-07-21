//! idb_backend.rs — IndexedDB-backed `commitments::Backend`, for wasm.
//!
//! `commit` reads the existing entry and writes inside one read-write
//! transaction: IDB serializes all access to a store within a
//! transaction, so read-then-write is atomic without a separate lock.

use idb::{Database, DatabaseEvent, Error as IdbError, ObjectStoreParams, TransactionMode};
use js_sys::{Date, Uint8Array};
use konduit_data::{Duration, Lock, Tag};
use wasm_bindgen::JsValue;

use super::{Backend, Commitment, Error};

const STORE: &str = "konduit-commitments";

impl From<IdbError> for Error {
    fn from(e: IdbError) -> Self {
        Error::Backend(format!("{e:?}"))
    }
}

fn to_js(bytes: &[u8]) -> JsValue {
    Uint8Array::from(bytes).into()
}

fn from_js(value: JsValue) -> Vec<u8> {
    Uint8Array::new(&value).to_vec()
}

fn wasm_now() -> Duration {
    Duration::from_millis(Date::now() as u64)
}

pub struct IdbBackend {
    db: Database,
    now: Box<dyn Fn() -> Duration>,
}

impl IdbBackend {
    pub async fn open(db_name: &str) -> Result<Self, Error> {
        Self::open_with_clock(db_name, wasm_now).await
    }

    pub async fn open_with_clock(
        db_name: &str,
        now: impl Fn() -> Duration + 'static,
    ) -> Result<Self, Error> {
        let mut factory = idb::Factory::new()?;
        let mut req = factory.open(db_name, Some(1))?;
        req.on_upgrade_needed(|event| {
            let db = event.database().unwrap();
            let mut params = ObjectStoreParams::new();
            params.auto_increment(false);
            db.create_object_store(STORE, params).unwrap();
        });
        let db = req.await?;
        Ok(Self {
            db,
            now: Box::new(now),
        })
    }
}

#[async_trait::async_trait(?Send)]
impl Backend for IdbBackend {
    async fn commit(&self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error> {
        let txn = self.db.transaction(&[STORE], TransactionMode::ReadWrite)?;
        let store = txn.object_store(STORE)?;
        let key = minicbor::to_vec(&lock).map_err(|e| Error::Backend(e.to_string()))?;
        let key_js = to_js(&key);

        let existing = store.get(key_js.clone())?.await?;

        match existing {
            None => {
                let commitment = Commitment::new(tag, index, (self.now)());
                let value =
                    minicbor::to_vec(&commitment).map_err(|e| Error::Backend(e.to_string()))?;
                store.add(&to_js(&value), Some(&key_js))?.await?;
            }
            Some(bytes) => {
                let prior: Commitment =
                    minicbor::decode(&from_js(bytes)).map_err(|e| Error::Backend(e.to_string()))?;
                if prior.tag() != &tag || prior.index() != index {
                    txn.commit()?.await?;
                    return Err(Error::Conflict);
                }
            }
        }

        txn.commit()?.await?;
        Ok(())
    }

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        let txn = self.db.transaction(&[STORE], TransactionMode::ReadOnly)?;
        let store = txn.object_store(STORE)?;
        let key = minicbor::to_vec(lock).map_err(|e| Error::Backend(e.to_string()))?;
        let raw = store.get(to_js(&key))?.await?;
        raw.map(|bytes| {
            minicbor::decode(&from_js(bytes)).map_err(|e| Error::Backend(e.to_string()))
        })
        .transpose()
    }

    async fn sweep_before(&self, threshold: Duration) -> Result<u64, Error> {
        let txn = self.db.transaction(&[STORE], TransactionMode::ReadWrite)?;
        let store = txn.object_store(STORE)?;

        let mut removed: u64 = 0;
        let mut cursor: Option<idb::Cursor> = store.open_cursor(None, None)?.await?;

        while let Some(c) = cursor {
            let value = c.value()?;
            let commitment: Commitment =
                minicbor::decode(&from_js(value)).map_err(|e| Error::Backend(e.to_string()))?;

            if *commitment.at() <= threshold {
                c.delete()?.await?;
                removed += 1;
            }

            cursor = c.advance(1)?.await?;
        }

        txn.commit()?.await?;
        Ok(removed)
    }
}

#[cfg(all(test, feature = "idb"))]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use konduit_data::Duration;
    use wasm_bindgen_test::*;

    use super::IdbBackend;
    use crate::commitments::backend_test_suite as suite;

    wasm_bindgen_test_configure!(run_in_service_worker);

    static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

    async fn temp_backend(clock: impl Fn() -> Duration + 'static) -> IdbBackend {
        let n = TEST_DB_COUNTER.fetch_add(1, Ordering::Relaxed);
        IdbBackend::open_with_clock(&format!("konduit-commitments-test-{n}"), clock)
            .await
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn fresh_insert_succeeds() {
        suite::fresh_insert_succeeds(&temp_backend(|| Duration::from_millis(0)).await).await;
    }

    #[wasm_bindgen_test]
    async fn conflicting_target_is_rejected() {
        suite::conflicting_target_is_rejected(&temp_backend(|| Duration::from_millis(0)).await)
            .await;
    }

    #[wasm_bindgen_test]
    async fn sweep_on_empty_store_is_noop() {
        suite::sweep_on_empty_store_is_noop(&temp_backend(|| Duration::from_millis(0)).await).await;
    }

    #[wasm_bindgen_test]
    async fn sweep_removes_only_old_entries() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn()).await;
        suite::sweep_removes_only_old_entries(&backend, &clock).await;
    }

    #[wasm_bindgen_test]
    async fn sweep_boundary_is_inclusive() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn()).await;
        suite::sweep_boundary_is_inclusive(&backend, &clock).await;
    }

    #[wasm_bindgen_test]
    async fn same_target_recommit_is_noop() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn()).await;
        suite::same_target_recommit_is_noop(&backend, &clock).await;
    }

    #[wasm_bindgen_test]
    async fn recommit_does_not_refresh_at() {
        let clock = suite::FakeClock::at(Duration::from_millis(0));
        let backend = temp_backend(clock.as_fn()).await;
        suite::recommit_does_not_refresh_at(&backend, &clock).await;
    }
}
