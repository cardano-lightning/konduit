use cardano_sdk::{Network, NetworkId};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Blockfrost,
    Utxorpc,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blockfrost => "blockfrost",
            Self::Utxorpc => "utxorpc",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Connector {
    Blockfrost(Blockfrost),
    UtxoRpc(UtxoRpc),
}

impl Connector {
    pub async fn connector(&self) -> anyhow::Result<crate::connector::Connector> {
        crate::connector::Connector::from_config(self).await
    }

    pub const fn network(&self) -> Network {
        match self {
            Connector::Blockfrost(blockfrost) => blockfrost.network,
            Connector::UtxoRpc(utxorpc) => utxorpc.network,
        }
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        Some(NetworkId::from(self.network()))
    }
}

impl fmt::Display for Connector {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connector : ")?;
        match self {
            Self::Blockfrost(inner) => write!(f, "{}", inner),
            Self::UtxoRpc(inner) => write!(f, "{}", inner),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blockfrost {
    pub network: Network,
    pub project_id: Option<String>,
}

impl Blockfrost {
    pub fn inferred_network(&self) -> anyhow::Result<Option<Network>> {
        let Some(project_id) = self.project_id.as_deref() else {
            return Ok(None);
        };

        [Network::Mainnet, Network::Preprod, Network::Preview]
            .into_iter()
            .find(|prefix| project_id.starts_with(&prefix.to_string()))
            .map(Some)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid Blockfrost project id: doesn't start with any known network?"
                )
            })
    }
}

impl fmt::Display for Blockfrost {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let project_id = if self.project_id.is_some() {
            "configured"
        } else {
            "unset"
        };

        write!(
            f,
            "Blockfrost || {} || project_id={project_id}",
            self.network
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoRpc {
    pub network: Network,
    pub uri: Option<String>,
}

impl fmt::Display for UtxoRpc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let uri = self.uri.as_deref().unwrap_or("unset");
        write!(f, "UTxO RPC || {} || uri={uri}", self.network)
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, Blockfrost, Connector, UtxoRpc};
    use cardano_sdk::{Network, NetworkId};

    #[test]
    fn backend_display_matches_explicit_selection() {
        assert_eq!(Backend::Blockfrost.to_string(), "blockfrost");
        assert_eq!(Backend::Utxorpc.to_string(), "utxorpc");
    }

    #[test]
    fn blockfrost_inferred_network_is_validated() {
        let config = Blockfrost {
            network: Network::Preprod,
            project_id: Some("preprod12345".to_string()),
        };

        let inferred = config.inferred_network().expect("project id should infer");

        assert_eq!(inferred, Some(Network::Preprod));
    }

    #[test]
    fn connector_network_id_comes_from_explicit_network() {
        let config = Connector::Blockfrost(Blockfrost {
            network: Network::Preview,
            project_id: None,
        });

        assert_eq!(config.network_id(), Some(NetworkId::TESTNET));
    }

    #[test]
    fn utxorpc_connector_reports_explicit_network_and_network_id() {
        let config = Connector::UtxoRpc(UtxoRpc {
            network: Network::Mainnet,
            uri: Some("http://127.0.0.1:1337".to_string()),
        });

        assert_eq!(config.network(), Network::Mainnet);
        assert_eq!(config.network_id(), Some(NetworkId::MAINNET));
    }
}
