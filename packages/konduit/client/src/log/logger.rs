use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use konduit_data::{Lock, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

// ---------- Id ----------

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Encode, Decode,
)]
pub struct Id(#[n(0)] u64);

impl Id {
    /// Constructing an `Id` that's actually safe to use as a log key is
    /// `Logger`'s job (`next_id`), not this type's. Kept `pub(crate)` so
    /// nothing outside the logger can mint an `Id` that wasn't assigned
    /// by it.
    pub(crate) fn new(millis_since_epoch: u64) -> Self {
        Self(millis_since_epoch)
    }

    fn now() -> Self {
        Self::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        )
    }

    /// Lossy — for rendering ("submitted at ...") only.
    pub fn as_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_millis(self.0)
    }
}

// ---------- Record / Body ----------

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Record {
    #[n(0)]
    id: Id,
    #[n(1)]
    body: Body,
}

impl Record {
    pub fn id(&self) -> Id {
        self.id
    }
    pub fn body(&self) -> &Body {
        &self.body
    }
}

/// Everything that gets logged: everything signed (`Cheque`, `Squash`,
/// `Reg`), plus L1 submissions (`Tx`) — signed by the wallet key
/// rather than a per-tag signer, and not scoped to any `Tag` at all.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Body {
    #[n(0)]
    Cheque(#[n(0)] Cheque),
    #[n(1)]
    Squash(#[n(0)] Squash),
    #[n(2)]
    Reg(#[n(0)] RegEvent),
    #[n(3)]
    Tx(#[n(0)] TxSubmission),
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Cheque {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    index: u64,
    #[n(2)]
    lock: Lock,
    #[n(3)]
    amount: u64,
    #[n(4)]
    timeout: konduit_data::Duration,
    #[n(5)]
    outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Squash {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct RegEvent {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TxSubmission {
    #[n(0)]
    txid: konduit_data::TxId,
    #[n(1)]
    outcome: Option<Outcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Outcome {
    #[n(0)]
    Ok(#[n(0)] konduit_data::Secret),
    #[n(1)]
    Ko(#[n(0)] String),
}

// ---------- Logger (per-backend trait) ----------

#[async_trait::async_trait]
pub trait Logger: Send + Sync {
    /// Append `body`, assigning it a fresh, strictly-monotonic `Id`.
    /// No rejection, no uniqueness checks against other records — the
    /// handler decides whether appending is appropriate by checking
    /// the getters first; the logger has no opinion.
    async fn append(&self, body: Body) -> Result<Id, Error>;

    /// Patch the outcome into the record at `id`. Overwrites whatever
    /// was there — the logger doesn't know what "already settled"
    /// means; that check is the handler's job too.
    async fn update_outcome(&self, id: Id, outcome: Outcome) -> Result<(), Error>;

    // -- fixed getter surface: same across every backend --
    async fn get(&self, id: Id) -> Result<Option<Record>, Error>;
    async fn get_by_lock(&self, lock: &Lock) -> Result<Option<Record>, Error>;
    async fn get_by_tag_and_index(&self, tag: &Tag, index: u64) -> Result<Vec<Record>, Error>;
    async fn all_for_tag(&self, tag: &Tag) -> Result<Vec<Record>, Error>;
    async fn latest_id(&self) -> Result<Option<Id>, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no record found at id")]
    NotFound,
    #[error("backend error: {0}")]
    Backend(String),
}

// ---------- Log (backend-agnostic handle) ----------

/// The type `app::State` actually holds. A cheap, cloneable handle
/// wrapping whichever `Logger` impl was constructed at startup — file,
/// sqlite, or idb. Nothing downstream of `State` needs to know which.
#[derive(Clone)]
pub struct Log(Arc<dyn Logger>);

impl Log {
    pub fn new(backend: Arc<dyn Logger>) -> Self {
        Self(backend)
    }
}

#[async_trait::async_trait]
impl Logger for Log {
    async fn append(&self, body: Body) -> Result<Id, Error> {
        self.0.append(body).await
    }
    async fn update_outcome(&self, id: Id, outcome: Outcome) -> Result<(), Error> {
        self.0.update_outcome(id, outcome).await
    }
    async fn get(&self, id: Id) -> Result<Option<Record>, Error> {
        self.0.get(id).await
    }
    async fn get_by_lock(&self, lock: &Lock) -> Result<Option<Record>, Error> {
        self.0.get_by_lock(lock).await
    }
    async fn get_by_tag_and_index(&self, tag: &Tag, index: u64) -> Result<Vec<Record>, Error> {
        self.0.get_by_tag_and_index(tag, index).await
    }
    async fn all_for_tag(&self, tag: &Tag) -> Result<Vec<Record>, Error> {
        self.0.all_for_tag(tag).await
    }
    async fn latest_id(&self) -> Result<Option<Id>, Error> {
        self.0.latest_id().await
    }
}
