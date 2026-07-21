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
//! The `at` field is a coarse pruning guide only — not a precise
//! timestamp of anything. Commits happen within minutes of each other;
//! pruning happens on the order of weeks. There is no need to keep
//! `at` accurate across re-commits; see "Commit semantics" below.
//!
//! ## Commit semantics
//!
//! `commit(lock, tag, index)`:
//! - No existing entry for `lock`: a new `Commitment` is stored, with
//!   `at` set to this backend's own idea of "now".
//! - Existing entry with the same `tag`/`index`: no-op. `at` is left
//!   untouched — there's nothing to gain by refreshing a coarse
//!   pruning guide on every re-commit, so `now()` isn't even called on
//!   this path.
//! - Existing entry with a different `tag`/`index`: `Error::Conflict`,
//!   existing entry untouched.
//!
//! Each `Backend` impl owns its own clock (`now()` / `wasm_now()` /
//! `system_now()`), injectable at construction for tests — see
//! `backend_test_suite::FakeClock`.

use konduit_data::{Duration, Lock, Tag};
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod in_memory;

#[cfg(all(not(target_family = "wasm"), feature = "json"))]
pub mod file_backend;

#[cfg(feature = "idb")]
pub mod idb_backend;

#[cfg(test)]
pub(crate) mod backend_test_suite;

/// The `(tag, index)` pair a lock is committed to, plus the time it was
/// recorded. `at` is insert time, not an expiry — see module docs.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Commitment {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    index: u64,
    #[n(2)]
    at: Duration,
}

impl Commitment {
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

    /// Same `tag`/`index` as `other` (ignoring `at`). Currently unused
    /// internally — each `Backend` impl compares `tag()`/`index()`
    /// directly against the incoming call's arguments, since `commit`
    /// no longer receives a full candidate `Commitment` to compare
    /// against. Kept as a public convenience.
    pub fn same_target(&self, other: &Commitment) -> bool {
        self.tag == other.tag && self.index == other.index
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lock already committed to a different tag/index")]
    Conflict,
    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait::async_trait(?Send)]
pub trait Backend {
    /// Insert or no-op; see module "Commit semantics".
    async fn commit(&self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error>;

    async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error>;

    /// Remove every entry with `at <= threshold`. `threshold` is the
    /// caller's call — this module has no retention policy. A safety
    /// net for entries never explicitly resolved, not the primary
    /// cleanup path. Returns the number of entries removed.
    async fn sweep_before(&self, threshold: Duration) -> Result<u64, Error>;
}

pub struct Commitments {
    backend: Box<dyn Backend>,
}

impl Commitments {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    pub async fn commit(&self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error> {
        self.backend.commit(lock, tag, index).await
    }

    pub async fn get(&self, lock: &Lock) -> Result<Option<Commitment>, Error> {
        self.backend.get(lock).await
    }

    pub async fn sweep_before(&self, threshold: Duration) -> Result<u64, Error> {
        self.backend.sweep_before(threshold).await
    }
}
