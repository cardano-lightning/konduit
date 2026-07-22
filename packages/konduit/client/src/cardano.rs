use anyhow::{Context, Result, anyhow};
use cardano_connector::CardanoConnector;
// ASSUMED: crate name, type name, and constructor shape below.
use cardano_connector_client as client;
#[cfg(feature = "direct")]
use cardano_connector_direct as direct;
#[cfg(feature = "utxorpc")]
use cardano_connector_utxorpc as utxorpc;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction,
    transaction::state::ReadyForSigning,
};
use http_client;
use std::collections::BTreeMap;

pub mod config;
pub use config::Config;

#[non_exhaustive]
pub enum Cardano {
    Client(Box<client::Connector<http_client::ReqwestTransport>>),
    #[cfg(feature = "direct")]
    Blockfrost(Box<direct::Blockfrost>),
    #[cfg(feature = "utxorpc")]
    UtxoRpc(Box<utxorpc::UtxoRpc>),
}

impl Cardano {
    pub async fn new(config: &Config) -> Result<Self> {
        match config {
            Config::Client(cfg) => Self::from_client(cfg).await,
            Config::Blockfrost(cfg) => Self::from_blockfrost(cfg),
            Config::UtxoRpc(cfg) => Self::from_utxorpc(cfg).await,
        }
    }

    async fn from_client(config: &config::Client) -> Result<Self> {
        // ASSUMED constructor shape — confirm the real one.
        let transport = http_client::ReqwestTransport::new(None);
        let http_client =
            http_client::Client::new(transport, http_client::JsonCodec, config.base_url.clone());
        let connector = client::Connector::new(http_client).await.with_context(|| {
            format!(
                "failed to connect to cardano-connector-client at {}",
                config.base_url
            )
        })?;

        let live = connector.network();
        anyhow::ensure!(
            live == config.network,
            "cardano-connector-client at {} reports network {live}, expected {}",
            config.base_url,
            config.network
        );

        Ok(Self::Client(Box::new(connector)))
    }
}

// =============================================================================
// Blockfrost ("direct")
// =============================================================================

#[cfg(feature = "direct")]
impl Cardano {
    fn from_blockfrost(config: &config::Blockfrost) -> Result<Self> {
        let project_id = config.project_id.as_deref().ok_or_else(|| {
            anyhow!("Cardano backend blockfrost requires KONDUIT_BLOCKFROST_PROJECT_ID")
        })?;

        if let Some(inferred) = config.inferred_network()? {
            anyhow::ensure!(
                config.network == inferred,
                "blockfrost project id network ({inferred}) does not match configured network ({})",
                config.network
            );
        }

        Ok(Self::Blockfrost(Box::new(direct::Blockfrost::new(
            project_id.to_string(),
        ))))
    }
}

#[cfg(not(feature = "direct"))]
impl Cardano {
    fn from_blockfrost(_: &config::Blockfrost) -> Result<Self> {
        Err(anyhow!(
            "Blockfrost backend requested but 'direct' feature is disabled"
        ))
    }
}

// =============================================================================
// UTxO RPC ("utxorpc")
// =============================================================================

#[cfg(feature = "utxorpc")]
impl Cardano {
    async fn from_utxorpc(config: &config::UtxoRpc) -> Result<Self> {
        let endpoint = config
            .uri
            .as_deref()
            .ok_or_else(|| anyhow!("Cardano backend utxorpc requires KONDUIT_UTXORPC_URI"))?;

        let runtime_config = utxorpc::Config::new(endpoint.to_string(), config.network);
        let connector = utxorpc::UtxoRpc::connect(runtime_config)
            .await
            .with_context(|| format!("failed to initialize UTxO RPC backend at {endpoint}"))?;

        connector
            .health()
            .await
            .with_context(|| format!("failed to reach UTxO RPC backend at {endpoint}"))?;

        let live = utxorpc::live_network(endpoint).await?;
        utxorpc::ensure_network_matches(config.network, live, endpoint)?;

        Ok(Self::UtxoRpc(Box::new(connector)))
    }
}

#[cfg(not(feature = "utxorpc"))]
impl Cardano {
    async fn from_utxorpc(_: &config::UtxoRpc) -> Result<Self> {
        Err(anyhow!(
            "UTxO RPC backend requested but 'utxorpc' feature is disabled"
        ))
    }
}

// =============================================================================
// Trait Delegation
// =============================================================================

macro_rules! delegate {
    ($self:ident, $c:ident => $target:expr) => {
        match $self {
            Self::Client($c) => $target,
            #[cfg(feature = "direct")]
            Self::Blockfrost($c) => $target,
            #[cfg(feature = "utxorpc")]
            Self::UtxoRpc($c) => $target,
        }
    };
}

impl CardanoConnector for Cardano {
    fn network(&self) -> Network {
        delegate!(self, c => c.network())
    }

    async fn health(&self) -> Result<String> {
        delegate!(self, c => c.health().await)
    }

    async fn protocol_parameters(&self) -> Result<ProtocolParameters> {
        delegate!(self, c => c.protocol_parameters().await)
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> Result<BTreeMap<Input, Output>> {
        delegate!(self, c => c.utxos_at(payment, delegation).await)
    }

    async fn submit(&self, transaction: &Transaction<ReadyForSigning>) -> Result<()> {
        delegate!(self, c => c.submit(transaction).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::Network;

    #[test]
    #[cfg(feature = "direct")]
    fn blockfrost_config_without_project_id_is_not_runnable() {
        let config = Config::Blockfrost(config::Blockfrost {
            network: Network::Mainnet,
            project_id: None,
        });

        let error = Cardano::from_blockfrost(match &config {
            Config::Blockfrost(c) => c,
            _ => unreachable!(),
        })
        .unwrap_err();

        assert!(error.to_string().contains("KONDUIT_BLOCKFROST_PROJECT_ID"));
    }

    #[test]
    #[cfg(feature = "utxorpc")]
    fn utxorpc_config_without_uri_is_not_runnable() {
        let config = config::UtxoRpc {
            network: Network::Preview,
            uri: None,
        };

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let error = runtime
            .block_on(Cardano::from_utxorpc(&config))
            .unwrap_err();

        assert!(error.to_string().contains("KONDUIT_UTXORPC_URI"));
    }

    #[test]
    #[cfg(feature = "direct")]
    fn blockfrost_network_mismatch_is_rejected() {
        let config = config::Blockfrost {
            network: Network::Preview,
            project_id: Some("preprod12345".to_string()),
        };

        let error = Cardano::from_blockfrost(&config).unwrap_err();
        assert!(error.to_string().contains("does not match"));
    }

    #[test]
    #[cfg(feature = "direct")]
    fn blockfrost_validation_accepts_matching_project_id() {
        let config = config::Blockfrost {
            network: Network::Preview,
            project_id: Some("preview12345".to_string()),
        };

        assert!(Cardano::from_blockfrost(&config).is_ok());
    }

    #[test]
    #[cfg(not(feature = "direct"))]
    fn blockfrost_disabled_feature_returns_error() {
        let config = config::Blockfrost {
            network: Network::Mainnet,
            project_id: Some("mainnet12345".to_string()),
        };

        let error = Cardano::from_blockfrost(&config).unwrap_err();
        assert!(error.to_string().contains("'direct' feature is disabled"));
    }

    #[test]
    #[cfg(not(feature = "utxorpc"))]
    fn utxorpc_disabled_feature_returns_error() {
        let config = config::UtxoRpc {
            network: Network::Mainnet,
            uri: Some("http://127.0.0.1:1337".to_string()),
        };

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let error = runtime
            .block_on(Cardano::from_utxorpc(&config))
            .unwrap_err();
        assert!(error.to_string().contains("'utxorpc' feature is disabled"));
    }
}
