//! # Commitments
//!
//! Commitments of payment are conditional on the revealing of a secret.
//!
//! Assumption: A merchant respects that the secret of a lock of an invoice
//! is proof of purchase.
//!
//! What is unsafe?
//!
//! 1. Loss of control of funds without proof of payment.
//! 2. Double commitment: Non mutually exclusive funds sharing a lock.
//!
//! In the happy path Consumer learns a secret in the subsequent squash proposal.
//! In the less happy path Consumer learns the secret from chain.
//! In either case, point 1 is addressed.
//!
//! To prevent double commitment, the lock -> (tag, index) must be recorded.
//! It is safe to "forget" the lock as soon as the corresponding commitments have resolved,
//! via either timeouts or discovery of secret.
//!
//! How about "recommitting": paying an invoice (secret learned), and then attempting to commit to paying the same
//! invoice?
//!
//! All major lightning implementations prevent double commitments internally.
//! This is no prevention to a malicious merchant, but it's unclear what is gained in this scenario.
//! It is equivalent to the merchant using public information and therefore there is no
//! expectation a payment will be received. The beneficiary is the server who already knows the secret.
//!
//! So then it is relevant only when the server is colluding with the node.
//! But the client will again learn secret and be able to present proof of payment.
//! And our assumption is that the merchant will recognize this (whatever that means: eg give me my
//! coffee).
//!
//! We suggest the Client should retain proof of purchase aka secret for "a long time".
//! Where the safe amount of time is in the wider context of the thing for which the client is paying.
//!
//! The `at` field allows for pruning of old commitments.

use std::collections::BTreeMap;
use std::sync::Mutex;

use konduit_data::{Duration, Lock, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
// FIXME :: fix
// #[cfg(feature = "wasm")]
// mod backend_idb;

/// The `(tag, index)` pair a lock is committed to, plus the time it was
/// recorded. `at` is insert time, not an expiry — see module docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Commitment {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    index: u64,
    #[n(2)]
    at: Duration,
}

impl Commitment {
    /// Records `tag`/`index` as committed now.
    pub fn new(tag: Tag, index: u64, at: Duration) -> Self {
        Self { tag, index, at }
    }
    pub fn tag(&self) -> &Tag {
        &self.tag
    }
    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn at(&self) -> &Duration {
        &self.at
    }
}

// ---------- Backend ----------

#[async_trait::async_trait]
pub trait Backend: Send + Sync {
    /// Insert `lock -> commitment`, atomically. Fails with
    /// `Error::KeyExists` if `lock` is already committed — to any
    /// commitment, including the same tag/index — since a lock's
    /// commitment must be set exactly once and never silently
    /// overwritten.
    ///
    /// `&self`, not `&mut self`: the whole point of this trait is
    /// atomicity under concurrent callers, so each backend owns its
    /// own interior synchronization (a lock, or the storage engine's
    /// native compare-and-swap/`INSERT ... ON CONFLICT`) rather than
    /// relying on a single external `&mut` to serialize every call.
    async fn insert(&self, lock: Lock, commitment: Commitment) -> Result<(), Error>;

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error>;

    /// Sweep every entry whose `at` is old enough that the protocol
    /// guarantees resolution (see [`MAX_COMMITMENT_WINDOW_SECS`]),
    /// regardless of whether an explicit resolution signal was ever
    /// received. Returns the number of entries removed. A safety net for
    /// entries that were never explicitly `remove`d — e.g. after a crash
    /// with a gap in event history.
    async fn drop_before(&self, threshold: Duration) -> Result<u64, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lock already committed")]
    KeyExists,
    #[error("backend error: {0}")]
    Backend(String),
}

// ---------- Commitments ----------

pub struct Commitments {
    backend: Box<dyn Backend>,
}

impl Commitments {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    pub async fn insert(&self, lock: Lock, commitment: Commitment) -> Result<(), Error> {
        self.backend.insert(lock, commitment).await
    }

    pub async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        self.backend.get(lock).await
    }

    /// Sweep everything past [`MAX_COMMITMENT_WINDOW_SECS`], using the
    /// current wall-clock time. Fallback safety net; see module docs.
    pub async fn drop_before(&self, threshold: Duration) -> Result<u64, Error> {
        self.backend.drop_before(threshold).await
    }
}

// ---------- In-memory backend (tests) ----------

/// Reference backend for tests. Correct, not fast — one global mutex
/// around a plain map. `entry().or_insert()` gives the atomic
/// check-and-set for free: the mutex guard holds the whole
/// check-then-write as one critical section, so two concurrent
/// `insert`s on the same lock can't both observe "absent".
#[derive(Default)]
pub struct InMemory {
    entries: Mutex<BTreeMap<Lock, Commitment>>,
}

impl InMemory {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl Backend for InMemory {
    async fn insert(&self, lock: Lock, commitment: Commitment) -> Result<(), Error> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        match entries.entry(lock) {
            std::collections::btree_map::Entry::Occupied(_) => Err(Error::KeyExists),
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(commitment);
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

    async fn drop_before(&self, threshold: Duration) -> Result<u64, Error> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| Error::Backend("poisoned".into()))?;
        let before = entries.len();
        entries.retain(|_, commitment| *commitment.at() > threshold);
        Ok((before - entries.len()) as u64)
    }
}
