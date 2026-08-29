use serde::{Deserialize, Serialize};

use crate::{connector, session::SubmitVia, waiter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cardano: connector::Config,
    pub wallet: cardano_wallet::Config,
    pub wait: waiter::Config,
    pub submit_via: SubmitVia,
    /// Where the CLI persists `tip` between runs.
    pub tip_cache_path: std::path::PathBuf,
    /// Where the CLI persists the addressbook between runs.
    pub addressbook_path: std::path::PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cardano: Default::default(),
            wallet: Default::default(),
            wait: Default::default(),
            submit_via: Default::default(),
            tip_cache_path: "/tmp/cardano-session-tip.json".into(),
            addressbook_path: "/tmp/cardano-session-addressbook.json".into(),
        }
    }
}
