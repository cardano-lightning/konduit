// commitments.rs — was payments.rs. `Cache` renamed `Commitments`; `Entries`,
// `Entry`, `Commit`, `Outcome`, `Status`, `Ko`, `Error`, `CommitError` all
// unchanged in substance.
use std::collections::BTreeMap;

use konduit_data::{Duration, Lock, Locked, Secret};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

// ---------- Commitments ----------

/// Record of payment attempts and their `Lock`s. Owns both indexes over
/// a tag's entries (`index -> Entry`, `Lock -> index`). An entry's
/// `lock` is fixed at creation and never changes (retries reuse it by
/// definition), so `locks` is only ever written in `insert` and removed
/// in `drop`, never touched by `retry`.
///
/// Almost everything here is safe to lose — a forgotten entry just
/// means re-attempting a payment. The one real hazard: `locks` must not
/// be lost between attempts, or a retry could sign a fresh `Locked`
/// reusing a `Lock` under a different `index`, which the server-side
/// partial order can't distinguish from an honest retry. If this is
/// lost, the safe recovery is to treat every in-flight `Lock` as
/// unusable rather than risk a collision.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Commitments {
    #[n(0)]
    entries: Entries,
    #[n(1)]
    locks: BTreeMap<Lock, u64>,
}

impl Commitments {
    pub fn insert(
        &mut self,
        at: Duration,
        pay_request: Vec<u8>,
        locked: Locked,
    ) -> Result<(), Error> {
        self.locks.insert(locked.lock().clone(), locked.index());
        self.entries.insert(Entry::new(at, pay_request, locked))?;
        Ok(())
    }

    pub fn retry(
        &mut self,
        index: u64,
        at: Duration,
        amount: u64,
        timeout: Duration,
    ) -> Result<(), Error> {
        self.entries
            .update(index, |entry| entry.retry(at, amount, timeout).map(|_| ()))
    }

    pub fn set_ok(&mut self, index: u64, at: Duration, secret: Secret) -> Result<(), Error> {
        self.entries.update(index, |entry| entry.set_ok(at, secret))
    }

    pub fn set_ko(&mut self, index: u64, at: Duration, ko: Ko) -> Result<(), Error> {
        self.entries.update(index, |entry| entry.set_ko(at, ko))
    }

    /// Drop the oldest `n` entries, unregistering the one lock each of
    /// them held, and returning those locks so the caller can propagate
    /// the cleanup up to `State`'s global registry.
    pub fn drop(&mut self, n: usize) -> Vec<Lock> {
        self.entries
            .drop(n)
            .into_iter()
            .map(|entry| {
                let lock = entry.lock();
                self.locks.remove(&lock);
                lock
            })
            .collect()
    }

    pub fn get(&self, index: u64) -> Option<&Entry> {
        self.entries.find(index)
    }

    pub fn get_by_lock(&self, lock: &Lock) -> Option<&Entry> {
        self.entries.find(*self.locks.get(lock)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// Structural invariants this type can't enforce against arbitrary
    /// `Decode` input: every entry validates on its own terms, and the
    /// `locks` index agrees exactly with `entries` — same lock, same
    /// index, one entry each, no orphans on either side.
    pub fn validate(&self) -> Result<(), Error> {
        for entry in self.entries.iter() {
            entry.validate()?;
            if self.locks.get(&entry.lock()) != Some(&entry.index()) {
                return Err(Error::LockIndexMismatch);
            }
        }
        if self.locks.len() != self.entries.iter().count() {
            return Err(Error::LockIndexMismatch);
        }
        Ok(())
    }

    /// Decode `Commitments` from CBOR bytes and validate it, and every
    /// `Entry` it contains, before handing it back.
    pub fn decode_validated(bytes: &[u8]) -> Result<Self, Error> {
        let commitments: Commitments = minicbor::decode(bytes)?;
        commitments.validate()?;
        Ok(commitments)
    }
}

// ---------- Entries ----------

/// Plain `Vec<Entry>` wrapper. Entries arrive in increasing-index
/// order (from a monotonic counter) and are dropped oldest-first, so a
/// `Vec` in insertion order needs no separate index structure — `find`
/// is a linear scan, which is fine at the sizes this is expected to
/// hold. If that stops being true, revisit for a keyed structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
struct Entries {
    #[n(0)]
    items: Vec<Entry>,
}

impl Entries {
    fn insert(&mut self, entry: Entry) -> Result<(), Error> {
        if self.find(entry.index()).is_some() {
            return Err(Error::DuplicateIndex);
        }
        self.items.push(entry);
        Ok(())
    }

    fn find(&self, index: u64) -> Option<&Entry> {
        self.items.iter().find(|e| e.index() == index)
    }

    /// Locate the entry at `index` and apply `f` to it, propagating
    /// whatever `f` itself returns. `Err(UnknownIndex)` if there's no
    /// entry there.
    fn update<T>(
        &mut self,
        index: u64,
        f: impl FnOnce(&mut Entry) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let entry = self
            .items
            .iter_mut()
            .find(|e| e.index() == index)
            .ok_or(Error::UnknownIndex)?;
        f(entry)
    }

    /// Remove and return the oldest `n` entries.
    fn drop(&mut self, n: usize) -> Vec<Entry> {
        let n = n.min(self.items.len());
        self.items.drain(0..n).collect()
    }

    fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.items.iter()
    }
}

// ---------- Entry ----------

/// One pay request, identified by its `Lock`, pursued through however
/// many `Commit`s it takes to redeem.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Entry {
    #[n(0)]
    pay_request: Vec<u8>,
    /// Index assigned
    #[n(2)]
    index: u64,
    /// Pay request lock
    #[n(3)]
    lock: Lock,
    #[n(4)]
    commits: Vec<Commit>,
}

impl Entry {
    /// A new entry always starts with a commit.
    /// This fixes the lock.
    /// Note we do not verify args coherence.
    /// For example: the correspondence between `pay_request` and `locked`,
    /// signature correctness.
    fn new(at: Duration, pay_request: Vec<u8>, locked: Locked) -> Self {
        Self {
            pay_request,
            index: locked.index(),
            lock: locked.lock().clone(),
            commits: vec![Commit::new(at, locked.amount(), locked.timeout())],
        }
    }

    // accessors
    pub fn pay_request(&self) -> &[u8] {
        &self.pay_request
    }
    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn lock(&self) -> Lock {
        self.lock
    }
    pub fn commits(&self) -> &[Commit] {
        &self.commits
    }
    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    /// Invariant "at least one commit" holds as long as this was built
    /// via `new`/`decode_validated`.
    pub fn last_commit(&self) -> &Commit {
        self.commits.last().expect("entry always has >=1 commit")
    }

    pub fn last_commit_mut(&mut self) -> &mut Commit {
        self.commits
            .last_mut()
            .expect("entry always has >=1 commit")
    }

    /// True once any commit has resolved Ok — at most one ever will.
    pub fn is_settled(&self) -> bool {
        self.commits.iter().any(Commit::is_ok)
    }

    pub fn settled_commit(&self) -> Option<&Commit> {
        self.commits.iter().find(|c| c.is_ok())
    }

    /// Sign and send a new commit against this entry's existing lock,
    /// with new `amount`/`timeout`, requested `at`.
    /// Only allowed when the last commit has been declared Ko and the
    /// entry isn't already settled.
    /// No `Lock` here, the caller is responsible.
    fn retry(
        &mut self,
        at: Duration,
        amount: u64,
        timeout: Duration,
    ) -> Result<&mut Commit, Error> {
        if self.is_settled() {
            return Err(Error::AlreadySettled);
        }
        if !self.last_commit().is_ko() {
            return Err(Error::NotKo);
        }
        self.commits.push(Commit::new(at, amount, timeout));
        Ok(self.commits.last_mut().unwrap())
    }

    /// Settle the last commit as Ok. A late secret always attaches to
    /// the last commit — once retried, earlier commits aren't
    /// reachable through this API at all.
    fn set_ok(&mut self, at: Duration, secret: Secret) -> Result<(), Error> {
        if self.is_settled() {
            return Err(Error::AlreadySettled);
        }
        self.last_commit_mut().set_ok(at, secret)?;
        Ok(())
    }

    /// Mark the last commit as Ko.
    fn set_ko(&mut self, at: Duration, ko: Ko) -> Result<(), Error> {
        if self.is_settled() {
            return Err(Error::AlreadySettled);
        }
        self.last_commit_mut().set_ko(at, ko)?;
        Ok(())
    }

    /// Structural invariants the type can't enforce against arbitrary
    /// `Decode` input.
    pub fn validate(&self) -> Result<(), Error> {
        if self.commits.is_empty() {
            return Err(Error::NoCommits);
        }
        if self.commits.iter().filter(|c| c.is_ok()).count() > 1 {
            return Err(Error::MultipleOk);
        }
        Ok(())
    }

    /// Decode an `Entry` from CBOR bytes and validate it in isolation.
    /// Can't check lock-reuse against other entries — `Commitments::validate`
    /// is what reconciles that.
    pub fn decode_validated(bytes: &[u8]) -> Result<Self, Error> {
        let entry: Entry = minicbor::decode(bytes)?;
        entry.validate()?;
        Ok(entry)
    }
}

// ---------- Commit ----------

/// A locked cheque sent to `.../commit` is a treated with care.
/// Accommodates the possibility that the outcome is yet to be
/// established, and that an entry may need more than one attempt to
/// be successful
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Commit {
    /// Time commit requested
    #[n(0)]
    at: Duration,
    /// Amount on the locked
    #[n(1)]
    amount: u64,
    /// Timeout on the locked
    #[n(2)]
    timeout: Duration,
    /// None if no result yet established.
    #[n(3)]
    outcome: Option<Outcome>,
}

impl Commit {
    /// A brand-new commit always starts with no outcome.
    fn new(at: Duration, amount: u64, timeout: Duration) -> Self {
        Self {
            at,
            amount,
            timeout,
            outcome: None,
        }
    }

    // accessors
    pub fn at(&self) -> Duration {
        self.at
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    pub fn is_pending(&self) -> bool {
        self.outcome.is_none()
    }

    pub fn is_ok(&self) -> bool {
        matches!(
            self.outcome,
            Some(Outcome {
                status: Status::Ok(_),
                ..
            })
        )
    }

    pub fn is_ko(&self) -> bool {
        matches!(
            self.outcome,
            Some(Outcome {
                status: Status::Ko(_),
                ..
            })
        )
    }

    /// Get the secret if this commit resolved Ok.
    pub fn secret(&self) -> Option<&Secret> {
        self.outcome.as_ref().and_then(|o| o.status.secret())
    }

    /// Get the Ko reason if this commit resolved Ko.
    pub fn ko(&self) -> Option<&Ko> {
        self.outcome.as_ref().and_then(|o| o.status.ko())
    }

    /// Settle this commit as Ok.
    /// No validation (lock is not in context).
    /// Overwrites are permitted: its a caller problem.
    fn set_ok(&mut self, at: Duration, secret: Secret) -> Result<(), CommitError> {
        self.outcome = Some(Outcome {
            at,
            status: Status::Ok(secret),
        });
        Ok(())
    }

    /// Mark commit as Ko.
    /// Overwrites are permitted if not Ok: its a caller problem.
    fn set_ko(&mut self, at: Duration, ko: Ko) -> Result<(), CommitError> {
        if self.is_ok() {
            return Err(CommitError::AlreadyOk);
        }
        self.outcome = Some(Outcome {
            at,
            status: Status::Ko(ko),
        });
        Ok(())
    }
}

// ---------- Outcome / Status ----------

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Outcome {
    /// Outcome established at:
    #[n(0)]
    at: Duration,
    #[n(1)]
    status: Status,
}

impl Outcome {
    pub fn at(&self) -> Duration {
        self.at
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Status {
    /// Pay Ok
    #[n(0)]
    Ok(#[n(0)] Secret),
    /// Pay Ko (aka failed).
    #[n(1)]
    Ko(#[n(0)] Ko),
}

impl Status {
    pub fn is_ok(&self) -> bool {
        matches!(self, Status::Ok(_))
    }
    pub fn is_ko(&self) -> bool {
        matches!(self, Status::Ko(_))
    }

    pub fn secret(&self) -> Option<&Secret> {
        match self {
            Status::Ok(s) => Some(s),
            Status::Ko(_) => None,
        }
    }

    pub fn ko(&self) -> Option<&Ko> {
        match self {
            Status::Ko(k) => Some(k),
            Status::Ok(_) => None,
        }
    }
}

// ---------- Ko ----------

/// TODO: provide Ko types.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Ko {
    /// Any other reason reported with string
    #[n(0)]
    Any(#[n(0)] String),
}

// ---------- Errors ----------

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("entry has no commits")]
    NoCommits,
    #[error("entry has more than one commit settled Ok")]
    MultipleOk,
    #[error("last commit has not been declared Ko; cannot retry")]
    NotKo,
    #[error("entry already settled with a successful commit")]
    AlreadySettled,
    #[error("index already has an entry")]
    DuplicateIndex,
    #[error("no entry found for index")]
    UnknownIndex,
    #[error("lock index is inconsistent with entries")]
    LockIndexMismatch,
    #[error(transparent)]
    Commit(#[from] CommitError),
    #[error(transparent)]
    Decode(#[from] minicbor::decode::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    #[error("commit is already settled Ok; cannot be overwritten")]
    AlreadyOk,
}
