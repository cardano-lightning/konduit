use anyhow::{Context, anyhow};
use cardano_connector::CardanoConnector;
use cardano_connector_direct::Blockfrost;
use cardano_connector_utxorpc::{
    Config as UtxoRpcConfig, UtxoRpc, ensure_network_matches, live_network,
};
use cardano_sdk::Network;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CardanoBackend {
    Blockfrost,
    Utxorpc,
}

impl fmt::Display for CardanoBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct CardanoArgs {
    #[arg(long, env = crate::env::CARDANO_BACKEND, default_value_t = CardanoBackend::Blockfrost)]
    pub backend: CardanoBackend,

    // Use blockfrost_project_id. The network is inferred, and the
    // URL is assumed to be blockfrost.io's
    #[arg(long, env = crate::env::BLOCKFROST_PROJECT_ID)]
    pub blockfrost_project_id: Option<String>,

    #[arg(long, env = crate::env::UTXORPC_URI)]
    pub utxorpc_uri: Option<String>,

    #[arg(long, env = crate::env::NETWORK)]
    pub network: Option<Network>,
}

impl CardanoArgs {
    pub async fn build(&self) -> anyhow::Result<super::Cardano> {
        match self.backend {
            CardanoBackend::Blockfrost => {
                let project_id = self.blockfrost_project_id()?;
                let client = Blockfrost::new(project_id.to_owned());
                client.health().await.with_context(|| {
                    format!(
                        "failed to reach configured Cardano backend {}",
                        CardanoBackend::Blockfrost.as_str()
                    )
                })?;

                Ok(super::Cardano::Blockfrost(client))
            }
            CardanoBackend::Utxorpc => {
                let config = self.utxorpc_config()?;
                let client = UtxoRpc::connect(config.clone()).await.with_context(|| {
                    format!(
                        "failed to initialize configured Cardano backend {} at {}",
                        CardanoBackend::Utxorpc.as_str(),
                        config.endpoint()
                    )
                })?;

                client.health().await.with_context(|| {
                    format!(
                        "failed to reach configured Cardano backend {} at {}",
                        CardanoBackend::Utxorpc.as_str(),
                        config.endpoint()
                    )
                })?;

                let live = live_network(config.endpoint()).await?;
                ensure_network_matches(config.network(), live, config.endpoint())?;

                Ok(super::Cardano::UtxoRpc(Box::new(client)))
            }
        }
    }

    fn blockfrost_project_id(&self) -> anyhow::Result<&str> {
        self.blockfrost_project_id
            .as_deref()
            .filter(|project_id| !project_id.trim().is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "Cardano backend {} requires {}",
                    CardanoBackend::Blockfrost.as_str(),
                    crate::env::BLOCKFROST_PROJECT_ID
                )
            })
    }

    fn utxorpc_config(&self) -> anyhow::Result<UtxoRpcConfig> {
        let endpoint = self
            .utxorpc_uri
            .as_deref()
            .filter(|endpoint| !endpoint.trim().is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "Cardano backend {} requires {}",
                    CardanoBackend::Utxorpc.as_str(),
                    crate::env::UTXORPC_URI
                )
            })?;

        let network = self.network.ok_or_else(|| {
            anyhow!(
                "Cardano backend {} requires {}",
                CardanoBackend::Utxorpc.as_str(),
                crate::env::NETWORK
            )
        })?;

        Ok(UtxoRpcConfig::new(endpoint.to_owned(), network))
    }
}

impl CardanoBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blockfrost => "blockfrost",
            Self::Utxorpc => "utxorpc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CardanoArgs, CardanoBackend};
    use cardano_connector_utxorpc::ensure_network_matches;
    use cardano_sdk::Network;

    #[test]
    fn blockfrost_backend_requires_project_id() {
        let args = CardanoArgs {
            backend: CardanoBackend::Blockfrost,
            blockfrost_project_id: None,
            utxorpc_uri: None,
            network: None,
        };

        let error = args
            .blockfrost_project_id()
            .expect_err("missing project id should fail");

        assert!(error.to_string().contains("KONDUIT_BLOCKFROST_PROJECT_ID"));
    }

    #[test]
    fn utxorpc_backend_requires_uri_and_network() {
        let args = CardanoArgs {
            backend: CardanoBackend::Utxorpc,
            blockfrost_project_id: None,
            utxorpc_uri: None,
            network: None,
        };

        let error = args
            .utxorpc_config()
            .expect_err("missing UTxO RPC config should fail");

        assert!(error.to_string().contains("KONDUIT_UTXORPC_URI"));
    }

    #[test]
    fn utxorpc_network_mismatch_is_rejected() {
        let error =
            ensure_network_matches(Network::Preview, Network::Preprod, "http://127.0.0.1:1337")
                .expect_err("network mismatch should fail");

        assert!(error.to_string().contains("does not match"));
    }
}
