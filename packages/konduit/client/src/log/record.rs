use konduit_data::{ChequeBody, Lock, Secret, SquashBody, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::Id;

// ---------- Deferred<A, C> ----------

/// An `A` logged now, whose `O` may only become known later. Just a
/// shape two or more `Body` variants happen to share right now
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Deferred<A, Ok, Ko> {
    #[n(0)]
    action: A,
    #[n(1)]
    outcome: Option<Outcome<Ok, Ko>>,
}

impl<A, Ok, Ko> Deferred<A, Ok, Ko> {
    pub fn new(action: A) -> Self {
        Self {
            action,
            outcome: None,
        }
    }
    pub fn action(&self) -> &A {
        &self.action
    }
    pub fn outcome(&self) -> Option<&Outcome<Ok, Ko>> {
        self.outcome.as_ref()
    }
    pub fn set_outcome(&mut self, outcome: Outcome<Ok, Ko>) {
        self.outcome = Some(outcome);
    }
    pub fn set_ok(&mut self, ok: Ok) {
        self.outcome = Some(Outcome::Ok(ok));
    }
    pub fn set_ko(&mut self, ko: Ko) {
        self.outcome = Some(Outcome::Ko(ko));
    }
}

/// Result, without assuming Ko impls Error.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Outcome<Ok, Ko> {
    #[n(0)]
    Ok(#[n(0)] Ok),
    #[n(1)]
    Ko(#[n(0)] Ko),
}

/// Placeholder generic Ko,
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Ko {}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Record {
    #[n(0)]
    id: Id,
    #[n(1)]
    body: Body,
}

impl Record {
    pub fn id(&self) -> super::Id {
        self.id
    }
    pub fn body(&self) -> &Body {
        &self.body
    }
}

/// Everything logged. Each variant here currently happens to be
/// `Deferred<A, C>` over its own distinct `C` — that's a fact about
/// these four, not a rule about `Body`.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Body {
    #[n(0)]
    Cheque(#[n(0)] Deferred<ChequeBody, Secret, Ko>),
    #[n(1)]
    Squash(#[n(0)] Deferred<SquashBody, (), Ko>),
    #[n(2)]
    Reg(#[n(0)] Deferred<(), (), Ko>),
    #[n(3)]
    Tx(#[n(0)] Deferred<Vec<u8>, [u8; 32], Ko>),
}

// ---------- Backend ----------

#[async_trait::async_trait]
pub trait Backend: Send + Sync {
    async fn store(&mut self, record: Record) -> Result<(), Error>;

    /// The backend matches `outcome`'s variant against the stored
    /// `Body`'s variant and errors on mismatch — it's the one place
    /// that actually has both in hand to check.
    async fn resolve<Ok, Ko>(&mut self, id: Id, outcome: Outcome<Ok, Ko>) -> Result<(), Error>;
    async fn resolve_ok<Ok, Ko>(&mut self, id: Id, ok: Ok) -> Result<(), Error>;
    async fn resolve_ko<Ok, Ko>(&mut self, id: Id, ko: Ko) -> Result<(), Error>;

    async fn get(&self, id: Id) -> Result<Option<Record>, Error>;
    async fn get_cheque_by_tag_index(&self, tag: &Tag, index: u64) -> Result<Vec<Record>, Error>;

    async fn cheques_with_lock(&self, lock: &Lock) -> Result<Option<Record>, Error>;
    async fn with_tag(&self, tag: &Tag) -> Result<Vec<Record>, Error>;
    async fn latest_id(&self) -> Result<Option<Id>, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no record found at id")]
    NotFound,
    #[error("outcome variant doesn't match the logged action's variant")]
    Mismatch,
    #[error("backend error: {0}")]
    Backend(String),
}

// ---------- Logger ----------

pub struct Logger {
    backend: Box<dyn Backend>,
    last: Id,
}

impl Logger {
    pub async fn new(backend: Box<dyn Backend>) -> Result<Self, Error> {
        let last = backend.latest_id().await?.unwrap_or(Id::new(0));
        Ok(Self { backend, last })
    }

    pub async fn append(&mut self, body: Body) -> Result<Id, Error> {
        let id = self.next_id();
        self.backend.store(Record { id, body }).await?;
        Ok(id)
    }

    pub async fn resolve(&mut self, id: Id, outcome: outcome) -> Result<(), Error> {
        self.backend.resolve(id, outcome).await
    }

    pub async fn get(&self, id: Id) -> Result<Option<Record>, Error> {
        self.backend.get(id).await
    }
    pub async fn get_by_lock(&self, lock: &Lock) -> Result<Option<Record>, Error> {
        self.backend.get_by_lock(lock).await
    }
    pub async fn get_by_tag_and_index(&self, tag: &Tag, index: u64) -> Result<Vec<Record>, Error> {
        self.backend.get_by_tag_and_index(tag, index).await
    }
    pub async fn all_for_tag(&self, tag: &Tag) -> Result<Vec<Record>, Error> {
        self.backend.all_for_tag(tag).await
    }
    pub fn latest_id(&self) -> Id {
        self.last
    }
}
