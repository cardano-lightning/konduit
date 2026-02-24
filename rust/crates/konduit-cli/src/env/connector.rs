use crate::{
    config::connector::{Blockfrost, Connector},
    shared::Fill,
};
use anyhow::anyhow;
use cardano_sdk::{Network, NetworkId};
use serde::{Deserialize, Serialize};

const ENV_BLOCKFROST_PROJECT_ID: &str = "KONDUIT_BLOCKFROST_PROJECT_ID";
const ENV_NETWORK: &str = "KONDUIT_NETWORK";

/// Connector options
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ConnectorEnv {
    /// Network. This is the fallback when blockfrost project is isn't specified.
    #[arg(long, default_value_t = Network::Mainnet, env = ENV_NETWORK)]
    #[serde(rename = "KONDUIT_NETWORK")]
    pub network: Network,

    #[arg(long, env = ENV_BLOCKFROST_PROJECT_ID, alias = "blockfrost")]
    #[serde(rename = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost_project_id: Option<String>,
}

impl TryFrom<ConnectorEnv> for Connector {
    type Error = anyhow::Error;

    fn try_from(env: ConnectorEnv) -> Result<Self, Self::Error> {
        if let Some(project_id) = env.blockfrost_project_id {
            return Ok(Connector::Blockfrost(Blockfrost { project_id }));
        };

        Err(anyhow::anyhow!(
            "Unable to deduce connector config. Possibly missing variables"
        ))
    }
}

impl Fill for ConnectorEnv {
    type Error = anyhow::Error;

    fn fill(self) -> anyhow::Result<Self> {
        let blockfrost = self.blockfrost_project_id;

        let network = self.network;

        if let Some(project_id) = blockfrost {
            let inferred_network = [Network::Mainnet, Network::Preprod, Network::Preview]
                .into_iter()
                .find(|prefix| project_id.starts_with(&prefix.to_string()))
                .ok_or(anyhow!(
                    "invalid Blockfrost project id: doesn't start with any known network?"
                ))?;

            if network != inferred_network {
                eprintln!(
                    "WARNING: inferred network from blockfrost project id differs from configured network; continuing with network={inferred_network}"
                );
            }

            return Ok(Self {
                blockfrost_project_id: Some(project_id),
                network: inferred_network,
            });
        }

        Ok(Self::placeholder(network))
    }
}

impl ConnectorEnv {
    pub fn placeholder(network: Network) -> Self {
        Self {
            network,
            blockfrost_project_id: Some(format!("{network}XXXXXXXXXXXXXXXXXXXX")),
        }
    }

    pub fn network_id(&self) -> NetworkId {
        self.blockfrost_project_id
            .as_ref()
            .map(|s| {
                if s.starts_with(&Network::Mainnet.to_string()) {
                    NetworkId::MAINNET
                } else {
                    NetworkId::TESTNET
                }
            })
            .unwrap_or(NetworkId::from(self.network))
    }
}
