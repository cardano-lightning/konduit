use cardano_tx_builder::NetworkId;
use serde::{Deserialize, Serialize};

use crate::config::{self, connector::Blockfrost};

/// Connector options
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ConnectorEnv {
    /// Connector option: Blockfrost project id
    #[arg(long)]
    #[serde(rename = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost: Option<String>,
}

impl ConnectorEnv {
    pub fn placeholder() -> Self {
        Self {
            blockfrost: Some("mainnetXXXXXXXXXXXXXXXXXXXX".to_string()),
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
        if let Some(project_id) = self.blockfrost {
            return Self {
                blockfrost: Some(project_id),
            };
        };
        Self::placeholder()
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        self.blockfrost.clone().map(|s| {
            if s.starts_with("mainnet") {
                NetworkId::MAINNET
            } else {
                NetworkId::TESTNET
            }
        })
    }
}
