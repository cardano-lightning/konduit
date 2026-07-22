use cardano_sdk::{Network, NetworkId};
use minicbor::{Decode, Encode};
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Config {
    /// Forward requests to a running `cardano-connector-client` service.
    /// The fallback when no specific backend is configured.
    #[n(0)]
    Client(#[n(0)] Client),
    #[n(1)]
    Blockfrost(#[n(0)] Blockfrost),
    #[n(2)]
    UtxoRpc(#[n(0)] UtxoRpc),
}

impl Config {
    pub const fn network(&self) -> Network {
        match self {
            Self::Client(inner) => inner.network,
            Self::Blockfrost(inner) => inner.network,
            Self::UtxoRpc(inner) => inner.network,
        }
    }

    pub fn network_id(&self) -> Option<NetworkId> {
        Some(NetworkId::from(self.network()))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::Client(Client::default())
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cardano : ")?;
        match self {
            Self::Client(inner) => write!(f, "{}", inner),
            Self::Blockfrost(inner) => write!(f, "{}", inner),
            Self::UtxoRpc(inner) => write!(f, "{}", inner),
        }
    }
}

/// Forwards every request over the wire to a running
/// `cardano-connector-client` instance rather than talking to a chain
/// backend directly. This is the always-available fallback: it has no
/// feature gate, since something has to work even when neither
/// `direct` nor `utxorpc` is compiled in.
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Client {
    #[n(0)]
    pub base_url: String,
    #[n(1)]
    pub network: Network,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:5555".to_string(),
            network: Network::Mainnet,
        }
    }
}

impl fmt::Display for Client {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Client || {} || base_url={}",
            self.network, self.base_url
        )
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Blockfrost {
    #[n(0)]
    pub network: Network,
    #[n(1)]
    pub project_id: Option<String>,
}

impl Blockfrost {
    pub fn new(network: Network, project_id: Option<String>) -> Self {
        Self {
            network,
            project_id,
        }
    }

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

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UtxoRpc {
    #[n(0)]
    pub network: Network,
    #[n(1)]
    pub uri: Option<String>,
}

impl fmt::Display for UtxoRpc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let uri = self.uri.as_deref().unwrap_or("unset");
        write!(f, "UTxO RPC || {} || uri={uri}", self.network)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum Backend {
    Client,
    Blockfrost,
    Utxorpc,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Client => "client",
            Self::Blockfrost => "blockfrost",
            Self::Utxorpc => "utxorpc",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::{Network, NetworkId};

    #[test]
    fn backend_display_matches_explicit_selection() {
        assert_eq!(Backend::Client.to_string(), "client");
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
        let config = Config::Blockfrost(Blockfrost {
            network: Network::Preview,
            project_id: None,
        });

        assert_eq!(config.network_id(), Some(NetworkId::TESTNET));
    }

    #[test]
    fn utxorpc_connector_reports_explicit_network_and_network_id() {
        let config = Config::UtxoRpc(UtxoRpc {
            network: Network::Mainnet,
            uri: Some("http://127.0.0.1:1337".to_string()),
        });

        assert_eq!(config.network(), Network::Mainnet);
        assert_eq!(config.network_id(), Some(NetworkId::MAINNET));
    }

    #[test]
    fn client_connector_reports_explicit_network_and_network_id() {
        let config = Config::Client(Client {
            network: Network::Mainnet,
            base_url: "http://localhost:5555".to_string(),
        });

        assert_eq!(config.network(), Network::Mainnet);
        assert_eq!(config.network_id(), Some(NetworkId::MAINNET));
    }
}
