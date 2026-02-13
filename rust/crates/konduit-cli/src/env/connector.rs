use cardano_tx_builder::NetworkId;
use serde::{Deserialize, Serialize};

use crate::{
    config::{self, connector::Blockfrost},
    env::network::Network,
};

/// Connector options
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ConnectorEnv {
    /// Network. This is the fallback if cardano connector not.
    #[arg(long)]
    #[serde(rename = "KONDUIT_NETWORK")]
    pub network: Option<Network>,

    #[arg(long)]
    #[serde(rename = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost: Option<String>,
}

impl ConnectorEnv {
    pub fn placeholder(network: Option<Network>) -> Self {
        Self {
            network: network,
            blockfrost: Some(format!(
                "{}XXXXXXXXXXXXXXXXXXXX",
                network
                    .unwrap_or(Network::Mainnet)
                    .to_string()
                    .to_lowercase()
            )),
        }
    }

    pub fn to_config(self) -> anyhow::Result<config::connector::Connector> {
        if let Some(project_id) = self.blockfrost {
            return Ok(config::connector::Connector::Blockfrost(Blockfrost {
                project_id: project_id.clone(),
            }));
        };
        Err(anyhow::anyhow!(
            "Unable to deduce connector config. Possibly missing variables"
        ))
    }

    pub fn fill(self) -> Self {
        if self.blockfrost.is_some() {
            self
        } else {
            Self::placeholder(self.network)
        }
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        self.blockfrost
            .as_ref()
            .map(|s| {
                if s.starts_with("mainnet") {
                    NetworkId::MAINNET
                } else {
                    NetworkId::TESTNET
                }
            })
            .or(self.network.map(|s| {
                if s == Network::Mainnet {
                    NetworkId::MAINNET
                } else {
                    NetworkId::TESTNET
                }
            }))
    }
}
