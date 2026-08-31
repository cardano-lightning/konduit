//! Commits
//!
//! In this simple version we do not support subsuming existing commits.
//! On failure, a new payme is required.

use std::{collections::BTreeMap, sync::Mutex};

use konduit_data::Lock;

/// TODO: embelish to permit retries
#[derive(Debug, Clone)]
pub struct Commit();

impl Commit {
    pub fn new() -> Self {
        Self()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("lock already committed")]
    AlreadyCommitted,
    #[error("not committed")]
    NotCommitted,
}

pub struct Commits {
    commits: Mutex<BTreeMap<Lock, Commit>>,
}

impl Commits {
    pub fn new() -> Self {
        Self {
            commits: Default::default(),
        }
    }

    fn commits(&self) -> std::sync::MutexGuard<'_, BTreeMap<Lock, Commit>> {
        self.commits.lock().expect("commits state poisoned")
    }

    /// Fails if `lock` already has a commit
    pub fn insert(&self, lock: Lock, commit: Commit) -> Result<(), Error> {
        if self.commits().contains_key(&lock) {
            return Err(Error::AlreadyCommitted);
        }
        self.commits().insert(lock, commit);
        Ok(())
    }

    pub fn get(&self, lock: &Lock) -> Option<Commit> {
        self.commits().get(lock).cloned()
    }

    pub fn remove(&self, lock: &Lock) -> Result<Commit, Error> {
        self.commits().remove(lock).ok_or(Error::NotCommitted)
    }
}
