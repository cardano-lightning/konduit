//! backend_idb.rs — IndexedDB-backed `commitments::Backend`, for wasm.
//!
//! IDB's own `add()` (as opposed to `put()`) already refuses to
//! overwrite an existing key, which lines up with `insert`'s
//! semantics almost exactly — I'm leaning on that native
//! constraint-violation behavior for atomicity rather than
//! implementing check-then-write by hand.

use idb::{Database, Error as IdbError, ObjectStoreParams, TransactionMode};
use konduit_data::Lock;

use super::{Backend, Commitment, Error};

const STORE: &str = "commitments";

pub struct IdbBackend {
    db: Database,
}

impl IdbBackend {
    pub async fn open() -> Result<Self, Error> {
        let mut factory = idb::Factory::new().map_err(|e| Error::Backend(e.to_string()))?;
        let mut req = factory
            .open("konduit-commitments", Some(1))
            .map_err(|e| Error::Backend(e.to_string()))?;

        req.on_upgrade_needed(|event| {
            let db = event.database().unwrap();
            let mut params = ObjectStoreParams::new();
            params.auto_increment(false);
            db.create_object_store(STORE, params).unwrap();
        });

        let db = req.await.map_err(|e| Error::Backend(e.to_string()))?;
        Ok(Self { db })
    }
}

#[async_trait::async_trait(?Send)]
impl Backend for IdbBackend {
    async fn insert(&self, lock: Lock, commitment: Commitment) -> Result<(), Error> {
        let txn = self
            .db
            .transaction(&[STORE], TransactionMode::ReadWrite)
            .map_err(|e| Error::Backend(e.to_string()))?;
        let store = txn
            .object_store(STORE)
            .map_err(|e| Error::Backend(e.to_string()))?;

        let key = minicbor::to_vec(&lock).map_err(|e| Error::Backend(e.to_string()))?;
        let value = minicbor::to_vec(&commitment).map_err(|e| Error::Backend(e.to_string()))?;

        // `add`, not `put`: rejects with a ConstraintError if `key`
        // already exists, which is exactly `Error::KeyExists`.
        let result = store.add(&value, Some(&key)).await;
        txn.commit()
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;

        match result {
            Ok(_) => Ok(()),
            Err(IdbError::ConstraintError(_)) => Err(Error::KeyExists),
            Err(e) => Err(Error::Backend(e.to_string())),
        }
    }

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        let txn = self
            .db
            .transaction(&[STORE], TransactionMode::ReadOnly)
            .map_err(|e| Error::Backend(e.to_string()))?;
        let store = txn
            .object_store(STORE)
            .map_err(|e| Error::Backend(e.to_string()))?;

        let key = minicbor::to_vec(lock).map_err(|e| Error::Backend(e.to_string()))?;
        let raw = store
            .get(&key)
            .await
            .map_err(|e| Error::Backend(e.to_string()))?;

        raw.map(|bytes| minicbor::decode(&bytes).map_err(|e| Error::Backend(e.to_string())))
            .transpose()
    }
}
