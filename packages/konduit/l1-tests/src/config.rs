use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    account, adaptor, cardano, compact_toml,
    scenario::{self, l2_resolver},
    wait, wallet,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cardano: cardano::Config,
    pub wallet: wallet::Config,
    pub adaptor: adaptor::Config,
    pub accounts: Vec<account::Config>,
    #[serde(default)]
    pub scenario: scenario::Config,
    #[serde(default)]
    pub wait: wait::Config,
    #[serde(default)]
    pub l2_resolver: l2_resolver::Config,
}

impl Default for Config {
    fn default() -> Self {
        let accounts = (0..3).map(|i| account::Config::new(i)).collect();
        Self {
            cardano: Default::default(),
            wallet: Default::default(),
            adaptor: Default::default(),
            accounts,
            scenario: Default::default(),
            wait: Default::default(),
            l2_resolver: Default::default(),
        }
    }
}

impl Config {
    /// Reads and parses the config file at `path` into a `Config`.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&raw)
            .with_context(|| format!("failed to parse config at {}", path.display()))
    }

    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        let toml_str = compact_toml::pretty_compact_with(self, |p| {
            matches!(p, [a, b, ..] if a == "scenario" && b == "txs") && p.len() > 2
        })
        .context("failed to serialize default config")?;
        std::fs::write(path, toml_str)
            .with_context(|| format!("failed to write config to {}", path.display()))?;
        Ok(())
    }
}
