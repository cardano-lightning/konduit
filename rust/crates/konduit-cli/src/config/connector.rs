use cardano_tx_builder::NetworkId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Connector {
    Blockfrost(Blockfrost),
}

impl Connector {
    pub fn connector(&self) -> anyhow::Result<crate::connector::Connector> {
        match self {
            Connector::Blockfrost(Blockfrost { project_id }) => {
                crate::connector::Connector::new_blockfrost(project_id)
            }
        }
    }

    // Guess the network from the config
    pub fn network_id(&self) -> Option<NetworkId> {
        match self {
            Connector::Blockfrost(blockfrost) => Some(blockfrost.network_id()),
        }
    }
}

impl fmt::Display for Connector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connector : ")?;
        match self {
            Self::Blockfrost(inner) => write!(f, "{}", inner),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockfrost {
    pub project_id: String,
}

impl Blockfrost {
    // Guess the network from the project id
    pub fn network_id(&self) -> NetworkId {
        if self.project_id.starts_with("mainnet") {
            NetworkId::MAINNET
        } else {
            NetworkId::TESTNET
        }
    }
}

impl fmt::Display for Blockfrost {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Blockfrost || {}", self.network_id())
    }
}
