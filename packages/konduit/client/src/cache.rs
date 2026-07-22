//! # Cache
use std::collections::BTreeMap;

use konduit_data::Tag;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{l1, l2, server};

#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Cache {
    #[n(0)]
    l1: l1::Cache,
    /// Known konduit servers
    #[n(1)]
    l2s: BTreeMap<Tag, l2::Cache>,
}

impl Cache {
    pub fn l1(&self) -> &l1::Cache {
        &self.l1
    }

    pub fn l1_mut(&mut self) -> &mut l1::Cache {
        &mut self.l1
    }

    pub fn l2(&self, tag: &Tag) -> Option<&l2::Cache> {
        self.l2s.get(tag)
    }

    pub fn add_l2(&mut self, tag: Tag) {
        self.l2s.insert(tag, l2::Cache::default());
    }

    pub fn l2_mut(&mut self, tag: &Tag) -> Option<&mut l2::Cache> {
        self.l2s.get_mut(tag)
    }

    pub fn remove_l2(&mut self, tag: &Tag) -> bool {
        self.l2s.remove(tag).is_some()
    }

    #[cfg(feature = "json")]
    pub fn show(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }
}

#[cfg(feature = "json")]
mod persistence {
    use std::fs;
    use std::path::Path;

    use super::Cache;

    impl Cache {
        /// Load from `path`, or return a fresh default Cache if it doesn't exist yet.
        pub fn load(path: &Path) -> anyhow::Result<Self> {
            if !path.exists() {
                return Ok(Self::default());
            }
            let text = fs::read_to_string(path)?;
            Ok(toml::from_str(&text)?)
        }

        /// Write the Cache to `path`, creating or overwriting it.
        pub fn save(&self, path: &Path) -> anyhow::Result<()> {
            let text = toml::to_string_pretty(self)?;
            fs::write(path, text)?;
            Ok(())
        }
    }
}
