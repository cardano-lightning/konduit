use cardano_connector_direct::Blockfrost;
use cardano_sdk::Network;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Config {
    Blockfrost { key: String, network: Network },
}

impl Config {
    pub fn build(&self) -> Blockfrost {
        match self {
            Config::Blockfrost { key, .. } => Blockfrost::new(key.clone()),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::Blockfrost {
            key: "mainnetxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            network: Network::Mainnet,
        }
    }
}
