//! What this rail has committed, keyed by lock.
//!
//! The lock is the idempotency key: repeated commits under one lock pay
//! at most once, and a repeat gets back the outcome of the first rather
//! than an error. An entry is kept at least until its commitment
//! timeout, because `reveal` has to answer for exactly that window.
//!
//! Durable when configured with a path, because that window outlives a
//! process: a restart between committing outbound and revealing the
//! secret inbound would otherwise lose the secret, leaving money paid
//! out with nothing claimed back. The whole map is rewritten on every
//! change — it holds only what is still in its timeout window, so it
//! stays small — and the write goes to a temporary file that is then
//! renamed, so a crash mid-write leaves the previous state rather than
//! half of this one.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use konduit_data::{Duration, Lock, Secret};
use konduit_rail::{Outcome, PostProblem, Resolution};
use serde::{Deserialize, Serialize};

/// Where the rail keeps what it has committed.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// None keeps them in memory only, which loses a secret to a
    /// restart between a commit and its reveal.
    pub path: Option<PathBuf>,
}

/// How far an outbound commitment got.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    /// Sent and unresolved.
    InFlight,
    Resolved(Secret),
    Failed(PostProblem),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Index of the outbound cheque, for matching against the receipt.
    pub index: u64,
    pub amount: u64,
    /// Absolute: when the outbound commitment elapses.
    pub timeout: Duration,
    pub state: State,
}

impl Commit {
    pub fn new(index: u64, amount: u64, timeout: Duration) -> Self {
        Self {
            index,
            amount,
            timeout,
            state: State::InFlight,
        }
    }

    /// What to answer a commit that repeats this one.
    pub fn outcome(&self) -> Outcome {
        match &self.state {
            State::InFlight => Outcome::Pending,
            State::Resolved(secret) => Outcome::Success(*secret),
            State::Failed(problem) => Outcome::FailedPost(problem.clone()),
        }
    }

    /// What to answer a reveal of this lock. An unresolved commitment
    /// past its timeout is a failure; before it, it is still pending.
    pub fn resolution(&self, now: Duration) -> Resolution {
        match &self.state {
            State::Resolved(secret) => Resolution::Success(*secret),
            State::Failed(problem) => Resolution::Failed(problem.clone()),
            State::InFlight if self.timeout <= now => Resolution::Failed(
                PostProblem::OutboundTimeout(format!("outbound cheque {} elapsed", self.index)),
            ),
            State::InFlight => Resolution::Pending,
        }
    }
}

/// The answer to claiming a lock: either it is ours to commit under, or
/// someone already did and this is what came of it.
#[derive(Debug, Clone)]
pub enum Claim {
    Fresh,
    Existing(Outcome),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not committed")]
    NotCommitted,
    #[error("reading {0}: {1}")]
    Read(PathBuf, std::io::Error),
    #[error("{0} is not a commits file: {1}")]
    Corrupt(PathBuf, serde_json::Error),
}

#[derive(Default)]
pub struct Commits {
    commits: Mutex<BTreeMap<Lock, Commit>>,
    /// Where `commits` is mirrored, if anywhere.
    path: Option<PathBuf>,
}

impl Commits {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads whatever a previous run left, so the locks it is still
    /// answering for survive the restart. A path that does not exist yet
    /// is an empty store, not an error — that is a first run.
    pub fn from_config(config: &Config) -> Result<Self, Error> {
        let Some(path) = config.path.clone() else {
            return Ok(Self::new());
        };
        let commits = match std::fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|err| Error::Corrupt(path.clone(), err))?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(err) => return Err(Error::Read(path, err)),
        };
        Ok(Self {
            commits: Mutex::new(commits),
            path: Some(path),
        })
    }

    fn commits(&self) -> std::sync::MutexGuard<'_, BTreeMap<Lock, Commit>> {
        self.commits.lock().expect("commits state poisoned")
    }

    /// Mirrors the map to disk, under the lock the caller already holds
    /// so what lands is never a mix of two changes.
    ///
    /// A failed write is logged, not returned: the commitment it
    /// describes has already been made, and refusing the caller would
    /// not unmake it. What is lost is durability of this one change,
    /// which the next change rewrites anyway.
    fn persist(&self, commits: &BTreeMap<Lock, Commit>) {
        let Some(path) = self.path.as_deref() else {
            return;
        };
        if let Err(err) = Self::write(path, commits) {
            tracing::error!("persisting commits to {} failed: {err}", path.display());
        }
    }

    fn write(path: &Path, commits: &BTreeMap<Lock, Commit>) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(commits)?)?;
        std::fs::rename(&tmp, path)
    }

    /// Claims `lock` for a commit, or reports what the commit already
    /// under it came to. One lock acquisition, so two concurrent commits
    /// under one lock cannot both be `Fresh`.
    pub fn claim(&self, lock: Lock, commit: Commit) -> Claim {
        let mut commits = self.commits();
        match commits.get(&lock) {
            Some(existing) => Claim::Existing(existing.outcome()),
            None => {
                commits.insert(lock, commit);
                self.persist(&commits);
                Claim::Fresh
            }
        }
    }

    /// Gives up a claim, for a commit that failed before anything was
    /// sent: nothing is owed, and a retry under the same lock is free to
    /// proceed.
    pub fn release(&self, lock: &Lock) {
        let mut commits = self.commits();
        commits.remove(lock);
        self.persist(&commits);
    }

    pub fn resolve(&self, lock: &Lock, secret: Secret) -> Result<(), Error> {
        let mut commits = self.commits();
        let commit = commits.get_mut(lock).ok_or(Error::NotCommitted)?;
        commit.state = State::Resolved(secret);
        self.persist(&commits);
        Ok(())
    }

    pub fn fail(&self, lock: &Lock, problem: PostProblem) -> Result<(), Error> {
        let mut commits = self.commits();
        let commit = commits.get_mut(lock).ok_or(Error::NotCommitted)?;
        commit.state = State::Failed(problem);
        self.persist(&commits);
        Ok(())
    }

    pub fn get(&self, lock: &Lock) -> Option<Commit> {
        self.commits().get(lock).cloned()
    }

    /// Drops entries whose commitment timeout has passed. A resolved
    /// secret is cached for exactly as long as the commitment it
    /// unlocked, which is what the spec asks of a rail.
    pub fn prune(&self, now: Duration) {
        let mut commits = self.commits();
        let before = commits.len();
        commits.retain(|_, commit| commit.timeout > now);
        if commits.len() != before {
            self.persist(&commits);
        }
    }

    /// What a reveal of `lock` answers, for a lock this store knows.
    pub fn resolution_of(&self, lock: &Lock) -> Option<Resolution> {
        self.get(lock)
            .map(|commit| commit.resolution(Duration::from_secs(0)))
    }

    /// Locks still in flight, for the platform's resolver to poll.
    pub fn in_flight(&self) -> Vec<Lock> {
        self.commits()
            .iter()
            .filter(|(_, commit)| commit.state == State::InFlight)
            .map(|(lock, _)| *lock)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> Lock {
        Lock([3; 32])
    }

    fn commit() -> Commit {
        Commit::new(1, 1_000, Duration::from_secs(1_000))
    }

    /// The point of persisting: the secret a restart would otherwise
    /// lose is the one the rail owes inbound.
    #[test]
    fn a_resolved_secret_survives_a_reload() {
        let dir = std::env::temp_dir().join(format!("cln-commits-{}", std::process::id()));
        let config = Config {
            path: Some(dir.join("commits.json")),
        };
        let secret = Secret([9; 32]);

        let commits = Commits::from_config(&config).unwrap();
        assert!(matches!(commits.claim(lock(), commit()), Claim::Fresh));
        commits.resolve(&lock(), secret).unwrap();
        drop(commits);

        let reloaded = Commits::from_config(&config).unwrap();
        assert_eq!(
            reloaded.get(&lock()).map(|commit| commit.state),
            Some(State::Resolved(secret))
        );
        assert_eq!(
            reloaded.resolution_of(&lock()),
            Some(Resolution::Success(secret))
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A lock claimed before the restart is still claimed after it, so
    /// the commitment made under it cannot be made twice.
    #[test]
    fn a_claim_survives_a_reload() {
        let dir = std::env::temp_dir().join(format!("cln-claims-{}", std::process::id()));
        let config = Config {
            path: Some(dir.join("commits.json")),
        };

        let commits = Commits::from_config(&config).unwrap();
        assert!(matches!(commits.claim(lock(), commit()), Claim::Fresh));
        drop(commits);

        let reloaded = Commits::from_config(&config).unwrap();
        match reloaded.claim(lock(), commit()) {
            Claim::Existing(Outcome::Pending) => (),
            other => panic!("expected the claim to stand, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Without a path there is nothing to reload, which is the state the
    /// `TODO` this replaces described.
    #[test]
    fn without_a_path_nothing_is_kept() {
        let commits = Commits::from_config(&Config::default()).unwrap();
        assert!(matches!(commits.claim(lock(), commit()), Claim::Fresh));
        assert!(
            Commits::from_config(&Config::default())
                .unwrap()
                .get(&lock())
                .is_none()
        );
    }
}
