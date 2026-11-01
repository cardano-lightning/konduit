//! What this rail has committed, keyed by lock.
//!
//! The lock is the idempotency key: repeated commits under one lock pay
//! at most once, and a repeat gets back the outcome of the first rather
//! than an error. An entry is kept at least until its commitment
//! timeout, because `reveal` has to answer for exactly that window.

use std::{collections::BTreeMap, sync::Mutex};

use konduit_data::{Duration, Lock, Secret};
use konduit_rail::{Outcome, PostProblem, Resolution};

/// How far an outbound commitment got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Sent and unresolved.
    InFlight,
    Resolved(Secret),
    Failed(PostProblem),
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("not committed")]
    NotCommitted,
}

#[derive(Default)]
pub struct Commits {
    commits: Mutex<BTreeMap<Lock, Commit>>,
}

impl Commits {
    pub fn new() -> Self {
        Self::default()
    }

    fn commits(&self) -> std::sync::MutexGuard<'_, BTreeMap<Lock, Commit>> {
        self.commits.lock().expect("commits state poisoned")
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
                Claim::Fresh
            }
        }
    }

    /// Gives up a claim, for a commit that failed before anything was
    /// sent: nothing is owed, and a retry under the same lock is free to
    /// proceed.
    pub fn release(&self, lock: &Lock) {
        self.commits().remove(lock);
    }

    pub fn resolve(&self, lock: &Lock, secret: Secret) -> Result<(), Error> {
        let mut commits = self.commits();
        let commit = commits.get_mut(lock).ok_or(Error::NotCommitted)?;
        commit.state = State::Resolved(secret);
        Ok(())
    }

    pub fn fail(&self, lock: &Lock, problem: PostProblem) -> Result<(), Error> {
        let mut commits = self.commits();
        let commit = commits.get_mut(lock).ok_or(Error::NotCommitted)?;
        commit.state = State::Failed(problem);
        Ok(())
    }

    pub fn get(&self, lock: &Lock) -> Option<Commit> {
        self.commits().get(lock).cloned()
    }

    /// Drops entries whose commitment timeout has passed. A resolved
    /// secret is cached for exactly as long as the commitment it
    /// unlocked, which is what the spec asks of a rail.
    pub fn prune(&self, now: Duration) {
        self.commits().retain(|_, commit| commit.timeout > now);
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
