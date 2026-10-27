use crate::{
    config::connector::{Backend, Blockfrost, Connector, Node, UtxoRpc},
    shared::Fill,
};
use anyhow::anyhow;
use cardano_sdk::{Network, NetworkId, PublicNetwork};
use serde::{Deserialize, Serialize};

const ENV_CARDANO_BACKEND: &str = "KONDUIT_CARDANO_BACKEND";
const ENV_BLOCKFROST_PROJECT_ID: &str = "KONDUIT_BLOCKFROST_PROJECT_ID";
const ENV_UTXORPC_URI: &str = "KONDUIT_UTXORPC_URI";
const ENV_NODE_SOCKET: &str = "KONDUIT_NODE_SOCKET";
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

    #[arg(long, env = ENV_NODE_SOCKET)]
    #[serde(rename = "KONDUIT_NODE_SOCKET")]
    pub node_socket: Option<String>,
}

impl TryFrom<ConnectorEnv> for Connector {
    type Error = anyhow::Error;

    fn try_from(env: ConnectorEnv) -> Result<Self, Self::Error> {
        let env_public_network = env
            .network
            .and_then(|n| PublicNetwork::try_from(n).ok())
            .ok_or_else(|| {
                anyhow!(
                    "invalid network value for {}: must be one of mainnet, preprod, or preview",
                    ENV_NETWORK
                )
            });
        match env.backend {
            Backend::Blockfrost => {
                let inferred_public_network =
                    infer_blockfrost_network(env.blockfrost_project_id.as_deref());

                let public_network = match (env_public_network, inferred_public_network) {
                    (Ok(configured), Some(inferred)) if configured != inferred => Err(anyhow!(
                        "inferred network from blockfrost project id differs from configured network; please ensure {} matches the project id prefix",
                        ENV_NETWORK
                    )),
                    (Ok(configured), _) => Ok(configured),
                    (_, Some(inferred)) => Ok(inferred),
                    (Err(_), None) => Err(anyhow!(
                        "Cardano backend blockfrost requires {} representing a public network or a {} with a known network prefix",
                        ENV_NETWORK,
                        ENV_BLOCKFROST_PROJECT_ID
                    )),
                }?;
                Ok(Connector::Blockfrost(Blockfrost {
                    network: public_network,
                    project_id: normalize(env.blockfrost_project_id),
                }))
            }
            Backend::Utxorpc => {
                let public_network = env_public_network?;
                Ok(Connector::UtxoRpc(UtxoRpc {
                    network: public_network,
                    uri: normalize(env.utxorpc_uri),
                }))
            }
            Backend::Node => {
                let env_network = env
                    .network
                    .ok_or(anyhow!("Cardano backend node requires {}", ENV_NETWORK))?;
                let env_node_socket = env
                    .node_socket
                    .ok_or(anyhow!("Cardano backend node requires {}", ENV_NODE_SOCKET))?;
                Ok(Connector::Node(Node {
                    network: env_network,
                    socket: env_node_socket,
                }))
            }
        }
    }
}

impl Fill for ConnectorEnv {
    type Error = anyhow::Error;

    fn fill(self) -> anyhow::Result<Self> {
        let blockfrost_project_id_is_none = self.blockfrost_project_id.is_none();
        match self.backend {
            Backend::Blockfrost => Ok(Self {
                backend: self.backend,
                blockfrost_project_id: self.blockfrost_project_id,
                network: if self.network.is_none() && blockfrost_project_id_is_none {
                    Some(Network::PREPROD)
                } else {
                    None
                },
                node_socket: self.node_socket,
                utxorpc_uri: self.utxorpc_uri,
            }),
            Backend::Utxorpc => Ok(Self {
                backend: self.backend,
                blockfrost_project_id: self.blockfrost_project_id,
                network: self.network.or(Some(Network::PREPROD)),
                node_socket: self.node_socket,
                utxorpc_uri: self.utxorpc_uri,
            }),
            Backend::Node => Ok(Self {
                backend: self.backend,
                blockfrost_project_id: self.blockfrost_project_id,
                network: self.network.or(Some(Network::PREPROD)),
                node_socket: self
                    .node_socket
                    .or_else(|| {
                        std::env::var("CARDANO_NODE_SOCKET_PATH")
                            .ok()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    })
                    .or(Some("/tmp/node.socket".to_string())),
                utxorpc_uri: self.utxorpc_uri,
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
            (Backend::Node, None) => Ok(NetworkId::TESTNET),
        }
    }
}

fn normalize(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn infer_blockfrost_network(project_id: Option<&str>) -> Option<PublicNetwork> {
    let project_id = project_id?;

    [
        PublicNetwork::Mainnet,
        PublicNetwork::Preprod,
        PublicNetwork::Preview,
    ]
    .into_iter()
    .find(|prefix| project_id.starts_with(&prefix.to_string()))
}

#[cfg(test)]
mod tests {
    use super::ConnectorEnv;
    use crate::config::connector::{Backend, Connector};
    use cardano_sdk::PublicNetwork;

    #[test]
    fn try_from_keeps_utxorpc_network_and_uri() {
        let env = ConnectorEnv {
            backend: Backend::Utxorpc,
            network: Some(PublicNetwork::Preprod.into()),
            blockfrost_project_id: None,
            utxorpc_uri: Some("http://127.0.0.1:1337".to_string()),
            node_socket: None,
        };

        let connector = Connector::try_from(env).expect("connector config should build");

        match connector {
            Connector::UtxoRpc(config) => {
                assert_eq!(config.network, PublicNetwork::Preprod);
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
            node_socket: None,
        };

        let error = match Connector::try_from(env) {
            Ok(_) => panic!("missing network should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("KONDUIT_NETWORK"));
    }

    #[test]
    fn blockfrost_requires_network_or_project_id() {
        let env = ConnectorEnv {
            backend: Backend::Blockfrost,
            network: None,
            blockfrost_project_id: None,
            utxorpc_uri: None,
            node_socket: None,
        };

        let error = match Connector::try_from(env) {
            Ok(_) => panic!("missing network and project id should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("KONDUIT_BLOCKFROST_PROJECT_ID"));
    }

    #[test]
    fn try_from_builds_node_config() {
        let env = ConnectorEnv {
            backend: Backend::Node,
            network: Some(PublicNetwork::Preprod.into()),
            blockfrost_project_id: None,
            utxorpc_uri: None,
            node_socket: Some("/tmp/node.socket".to_string()),
        };

        let connector = Connector::try_from(env).expect("node config should build");
        match connector {
            Connector::Node(config) => {
                assert_eq!(config.network, PublicNetwork::Preprod.into());
                assert_eq!(config.socket, "/tmp/node.socket");
            }
            other => panic!("expected Node config, got {other:?}"),
        }
    }
}
