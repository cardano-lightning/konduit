//! The set of rails an adaptor supports, and the only place a name is
//! resolved to one.
//!
//! One rail per name: a rail manages any multi-instance notion
//! internally. `konduit` is reserved, and rejected here at construction
//! rather than at request time.

use crate::{DynRail, RESERVED_NAME, Rail};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("`{RESERVED_NAME}` is reserved for what no rail owns")]
    Reserved,
    #[error("rail already registered: {0}")]
    Duplicate(&'static str),
}

#[derive(Default)]
pub struct Registry {
    rails: BTreeMap<&'static str, Arc<dyn DynRail>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<R: Rail + 'static>(&mut self, rail: R) -> Result<(), Error> {
        self.insert_shared(Arc::new(rail))
    }

    /// For a rail something else also holds — a background task of its
    /// own, say.
    pub fn insert_shared<R: Rail + 'static>(&mut self, rail: Arc<R>) -> Result<(), Error> {
        if R::NAME == RESERVED_NAME {
            return Err(Error::Reserved);
        }
        if self.rails.contains_key(R::NAME) {
            return Err(Error::Duplicate(R::NAME));
        }
        self.rails.insert(R::NAME, rail);
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &Arc<dyn DynRail>)> {
        self.rails.iter().map(|(name, rail)| (*name, rail))
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn DynRail>> {
        self.rails.get(name)
    }

    /// The configured rails, as `AdaptorInfo` reports them so a consumer
    /// sees the set it picks from.
    pub fn names(&self) -> Vec<&'static str> {
        self.rails.keys().copied().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.rails.is_empty()
    }
}
