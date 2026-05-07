use crate::{
    config::connector::{Backend, Blockfrost, Connector, UtxoRpc},
    shared::Fill,
};
use anyhow::anyhow;
use cardano_sdk::{Network, NetworkId};
use serde::{Deserialize, Serialize};

const ENV_CARDANO_BACKEND: &str = "KONDUIT_CARDANO_BACKEND";
const ENV_BLOCKFROST_PROJECT_ID: &str = "KONDUIT_BLOCKFROST_PROJECT_ID";
const ENV_UTXORPC_URI: &str = "KONDUIT_UTXORPC_URI";
const ENV_NETWORK: &str = "KONDUIT_NETWORK";

/// Connector options
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ConnectorEnv {
    #[arg(long, env = ENV_CARDANO_BACKEND, default_value_t = Backend::Blockfrost)]
    #[serde(rename = "KONDUIT_CARDANO_BACKEND")]
    pub backend: Backend,

    /// Cardano network selection.
    #[arg(long, env = ENV_NETWORK)]
    #[serde(rename = "KONDUIT_NETWORK")]
    pub network: Option<Network>,

    #[arg(long, env = ENV_BLOCKFROST_PROJECT_ID, alias = "blockfrost")]
    #[serde(rename = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost_project_id: Option<String>,

    #[arg(long, env = ENV_UTXORPC_URI, alias = "utxorpc")]
    #[serde(rename = "KONDUIT_UTXORPC_URI")]
    pub utxorpc_uri: Option<String>,
}

impl TryFrom<ConnectorEnv> for Connector {
    type Error = anyhow::Error;

    fn try_from(env: ConnectorEnv) -> Result<Self, Self::Error> {
        Ok(match env.backend {
            Backend::Blockfrost => Connector::Blockfrost(Blockfrost {
                network: env
                    .network
                    .or(infer_blockfrost_network(
                        env.blockfrost_project_id.as_deref(),
                    )?)
                    .unwrap_or(Network::Mainnet),
                project_id: normalize(env.blockfrost_project_id),
            }),
            Backend::Utxorpc => Connector::UtxoRpc(UtxoRpc {
                network: env
                    .network
                    .ok_or(anyhow!("Cardano backend utxorpc requires {}", ENV_NETWORK))?,
                uri: normalize(env.utxorpc_uri),
            }),
        })
    }
}

impl Fill for ConnectorEnv {
    type Error = anyhow::Error;

    fn fill(self) -> anyhow::Result<Self> {
        let blockfrost_project_id = normalize(self.blockfrost_project_id);
        let utxorpc_uri = normalize(self.utxorpc_uri);

        match self.backend {
            Backend::Blockfrost => {
                let inferred_network = infer_blockfrost_network(blockfrost_project_id.as_deref())?;
                let network = match (self.network, inferred_network) {
                    (Some(configured), Some(inferred)) if configured != inferred => {
                        eprintln!(
                            "WARNING: inferred network from blockfrost project id differs from configured network; continuing with network={inferred}"
                        );
                        Some(inferred)
                    }
                    (Some(configured), _) => Some(configured),
                    (None, Some(inferred)) => Some(inferred),
                    (None, None) => Some(Network::Mainnet),
                };

                Ok(Self {
                    backend: self.backend,
                    network,
                    blockfrost_project_id,
                    utxorpc_uri,
                })
            }
            Backend::Utxorpc => Ok(Self {
                backend: self.backend,
                network: Some(
                    self.network
                        .ok_or(anyhow!("Cardano backend utxorpc requires {}", ENV_NETWORK))?,
                ),
                blockfrost_project_id,
                utxorpc_uri,
            }),
        }
    }
}

impl ConnectorEnv {
    pub fn network_id(&self) -> anyhow::Result<NetworkId> {
        match (self.backend, self.network) {
            (_, Some(network)) => Ok(NetworkId::from(network)),
            (Backend::Blockfrost, None) => Ok(NetworkId::MAINNET),
            (Backend::Utxorpc, None) => {
                Err(anyhow!("Cardano backend utxorpc requires {}", ENV_NETWORK))
            }
        }
    }
}

fn normalize(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn infer_blockfrost_network(project_id: Option<&str>) -> anyhow::Result<Option<Network>> {
    let Some(project_id) = project_id else {
        return Ok(None);
    };

    [Network::Mainnet, Network::Preprod, Network::Preview]
        .into_iter()
        .find(|prefix| project_id.starts_with(&prefix.to_string()))
        .map(Some)
        .ok_or(anyhow!(
            "invalid Blockfrost project id: doesn't start with any known network?"
        ))
}

#[cfg(test)]
mod tests {
    use super::ConnectorEnv;
    use crate::config::connector::{Backend, Connector};
    use crate::shared::Fill;
    use cardano_sdk::{Network, NetworkId};

    #[test]
    fn fill_keeps_missing_blockfrost_project_id_unset() {
        let env = ConnectorEnv {
            backend: Backend::Blockfrost,
            network: Some(Network::Preview),
            blockfrost_project_id: None,
            utxorpc_uri: None,
        };

        let filled = env.fill().expect("fill should succeed");

        assert_eq!(filled.blockfrost_project_id, None);
        assert_eq!(filled.network, Some(Network::Preview));
    }

    #[test]
    fn try_from_keeps_utxorpc_network_and_uri() {
        let env = ConnectorEnv {
            backend: Backend::Utxorpc,
            network: Some(Network::Preprod),
            blockfrost_project_id: None,
            utxorpc_uri: Some("http://127.0.0.1:1337".to_string()),
        };

        let connector = Connector::try_from(env).expect("connector config should build");

        match connector {
            Connector::UtxoRpc(config) => {
                assert_eq!(config.network, Network::Preprod);
                assert_eq!(config.uri.as_deref(), Some("http://127.0.0.1:1337"));
            }
            other => panic!("expected UTxO RPC config, got {other:?}"),
        }
    }

    #[test]
    fn utxorpc_requires_explicit_network() {
        let env = ConnectorEnv {
            backend: Backend::Utxorpc,
            network: None,
            blockfrost_project_id: None,
            utxorpc_uri: Some("http://127.0.0.1:1337".to_string()),
        };

        let error = match Connector::try_from(env) {
            Ok(_) => panic!("missing network should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("KONDUIT_NETWORK"));
    }

    #[test]
    fn utxorpc_fill_ignores_stale_blockfrost_project_id() {
        let env = ConnectorEnv {
            backend: Backend::Utxorpc,
            network: Some(Network::Preview),
            blockfrost_project_id: Some("preprod12345".to_string()),
            utxorpc_uri: Some("http://127.0.0.1:1337".to_string()),
        };

        let filled = env.fill().expect("fill should succeed");

        assert_eq!(filled.network, Some(Network::Preview));
    }

    #[test]
    fn blockfrost_fill_infers_network_from_project_id() {
        let env = ConnectorEnv {
            backend: Backend::Blockfrost,
            network: None,
            blockfrost_project_id: Some("preview12345".to_string()),
            utxorpc_uri: None,
        };

        let filled = env.fill().expect("fill should infer Blockfrost network");

        assert_eq!(filled.network, Some(Network::Preview));
    }

    #[test]
    fn blockfrost_network_id_defaults_to_mainnet_when_unset() {
        let env = ConnectorEnv {
            backend: Backend::Blockfrost,
            network: None,
            blockfrost_project_id: None,
            utxorpc_uri: None,
        };

        assert_eq!(env.network_id().unwrap(), NetworkId::MAINNET);
    }
}
