use std::collections::BTreeMap;

use konduit_data::{Lock, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Commitments(#[n(0)] BTreeMap<Lock, (Tag, u64)>);

impl Commitments {
    pub fn get(&self, lock: &Lock) -> Option<(Tag, u64)> {
        self.0.get(lock).cloned()
    }

    /// Commit `lock` to `tag`. Errors if `lock` is already committed
    pub fn try_set(&mut self, lock: Lock, tag: Tag, index: u64) -> Result<(), Error> {
        if let Some(existing) = self.0.get(&lock) {
            return Err(Error::AlreadyCommitted {
                lock,
                existing: existing.clone(),
            });
        }
        self.0.insert(lock, (tag, index));
        Ok(())
    }

    pub fn remove(&mut self, lock: &Lock) {
        self.0.remove(lock);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lock already committed to tag {existing:?}")]
    AlreadyCommitted { lock: Lock, existing: (Tag, u64) },
}
