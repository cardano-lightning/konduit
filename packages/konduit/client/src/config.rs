//! # Config
//!
//! Pieces together the config of different components
use std::{collections::BTreeMap, path::Path};

use konduit_data::Tag;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{keys, l1, l2, server};

#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    #[n(0)]
    keys: keys::Config,
    /// L1 config
    #[n(1)]
    l1: l1::Config,
    /// Known konduit servers
    #[n(2)]
    servers: Vec<server::Config>,
    /// L2 configs which may or may not use a base_url from the known servers.
    #[n(3)]
    l2s: BTreeMap<Tag, l2::Config>,
}
impl Config {
    pub fn keys(&self) -> &keys::Config {
        &self.keys
    }

    pub fn keys_mut(&mut self) -> &mut keys::Config {
        &mut self.keys
    }

    /// Human-readable summary with secret material redacted, for `config show`.
    pub fn describe_redacted(&self) -> String {
        format!(
            "{}\nservers: {} configured\nl2s: {} configured",
            self.keys.describe_redacted(),
            self.servers.len(),
            self.l2s.len(),
        )
    }
}

#[cfg(feature = "toml")]
mod persistence {
    use std::fs;
    use std::path::Path;

    use super::Config;

    impl Config {
        /// Load from `path`, or return a fresh default config if it doesn't exist yet.
        pub fn load(path: &Path) -> anyhow::Result<Self> {
            if !path.exists() {
                return Ok(Self::default());
            }
            let text = fs::read_to_string(path)?;
            Ok(toml::from_str(&text)?)
        }

        /// Write the config to `path`, creating or overwriting it.
        pub fn save(&self, path: &Path) -> anyhow::Result<()> {
            let text = toml::to_string_pretty(self)?;
            fs::write(path, text)?;
            Ok(())
        }
    }
}
