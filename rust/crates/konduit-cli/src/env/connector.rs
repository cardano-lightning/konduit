use crate::{
    config::connector::{Blockfrost, Connector},
    env::network::Network,
    shared::Fill,
};
use cardano_tx_builder::NetworkId;
use serde::{Deserialize, Serialize};

/// Connector options
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ConnectorEnv {
    /// Network. This is the fallback if cardano connector not.
    #[arg(long, env = "KONDUIT_NETWORK", ignore_case = true)]
    #[serde(rename = "KONDUIT_NETWORK")]
    pub network: Option<Network>,

    #[arg(long, env = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    #[serde(rename = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost: Option<String>,
}

impl TryFrom<ConnectorEnv> for Connector {
    type Error = anyhow::Error;

    fn try_from(env: ConnectorEnv) -> Result<Self, Self::Error> {
        if let Some(project_id) = env.blockfrost {
            return Ok(Connector::Blockfrost(Blockfrost {
                project_id: project_id.clone(),
            }));
        };

        Err(anyhow::anyhow!(
            "Unable to deduce connector config. Possibly missing variables"
        ))
    }
}

impl Fill for ConnectorEnv {
    fn fill(self, global: ConnectorEnv) -> Self {
        let network = self.network.or(global.network);

        // In case where:
        //
        // - only the global option is passed
        // - but there's also a matching env var
        //
        // Both will be Some -- possibly with different values. And the local will default to the
        // env var, and override the global option; which is not expected. So in case they're both
        // some and different, we fallback to whichever differs from the env var.
        if self.blockfrost != global.blockfrost
            && self.blockfrost.is_some()
            && global.blockfrost.is_some()
        {
            let blockfrost_env = std::env::var("KONDUIT_BLOCKFROST_PROJECT_ID").ok();
            return Self {
                blockfrost: if self.blockfrost == blockfrost_env {
                    global.blockfrost
                } else {
                    self.blockfrost
                },
                network,
            };
        }

        let blockfrost = self.blockfrost.or(global.blockfrost);

        if blockfrost.is_some() {
            return Self {
                network,
                blockfrost,
            };
        }

        Self::placeholder(network)
    }
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
