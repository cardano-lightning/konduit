use crate::config::connector::{
    Blockfrost as BlockfrostConfig, Connector as ConnectorConfig, UtxoRpc as UtxoRpcConfig,
};
use anyhow::{Context, Result, anyhow};
use cardano_connector::CardanoConnector;
use cardano_connector_direct::Blockfrost;
use cardano_connector_utxorpc::{
    Config as UtxoRpcRuntimeConfig, UtxoRpc, ensure_network_matches, live_network,
};
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction,
    transaction::state::ReadyForSigning,
};
use std::collections::BTreeMap;

pub enum Connector {
    Blockfrost(Blockfrost),
    UtxoRpc(Box<UtxoRpc>),
}

impl Connector {
    pub async fn from_config(config: &ConnectorConfig) -> anyhow::Result<Self> {
        match config {
            ConnectorConfig::Blockfrost(config) => Self::new_blockfrost(config),
            ConnectorConfig::UtxoRpc(config) => Self::new_utxorpc(config).await,
        }
    }

    fn new_blockfrost(config: &BlockfrostConfig) -> anyhow::Result<Self> {
        let project_id = validated_blockfrost_project_id(config)?;

        Ok(Self::Blockfrost(Blockfrost::new(project_id.to_string())))
    }

    async fn new_utxorpc(config: &UtxoRpcConfig) -> anyhow::Result<Self> {
        let endpoint = config
            .uri
            .as_deref()
            .ok_or_else(|| anyhow!("Cardano backend utxorpc requires KONDUIT_UTXORPC_URI"))?;

        let runtime_config = UtxoRpcRuntimeConfig::new(endpoint.to_string(), config.network);
        let connector = UtxoRpc::connect(runtime_config.clone())
            .await
            .with_context(|| format!("failed to initialize UTxO RPC backend at {endpoint}"))?;

        connector
            .health()
            .await
            .with_context(|| format!("failed to reach UTxO RPC backend at {endpoint}"))?;

        let live = live_network(endpoint).await?;
        ensure_network_matches(config.network, live, endpoint)?;

        Ok(Self::UtxoRpc(Box::new(connector)))
    }
}

fn validated_blockfrost_project_id(config: &BlockfrostConfig) -> anyhow::Result<&str> {
    let project_id = config.project_id.as_deref().ok_or_else(|| {
        anyhow!("Cardano backend blockfrost requires KONDUIT_BLOCKFROST_PROJECT_ID")
    })?;

    if let Some(inferred_network) = config.inferred_network()? {
        ensure_network_matches(config.network, inferred_network, "blockfrost project id")?;
    }

    Ok(project_id)
}

impl CardanoConnector for Connector {
    fn network(&self) -> Network {
        match self {
            Self::Blockfrost(c) => c.network(),
            Self::UtxoRpc(c) => c.network(),
        }
    }

    async fn health(&self) -> Result<String> {
        match self {
            Self::Blockfrost(c) => c.health().await,
            Self::UtxoRpc(c) => c.health().await,
        }
    }

    async fn protocol_parameters(&self) -> Result<ProtocolParameters> {
        match self {
            Self::Blockfrost(c) => c.protocol_parameters().await,
            Self::UtxoRpc(c) => c.protocol_parameters().await,
        }
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> Result<BTreeMap<Input, Output>> {
        match self {
            Self::Blockfrost(c) => c.utxos_at(payment, delegation).await,
            Self::UtxoRpc(c) => c.utxos_at(payment, delegation).await,
        }
    }

    async fn submit(&self, transaction: &Transaction<ReadyForSigning>) -> Result<()> {
        match self {
            Self::Blockfrost(c) => c.submit(transaction).await,
            Self::UtxoRpc(c) => c.submit(transaction).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Connector, validated_blockfrost_project_id};
    use crate::config::connector::{Blockfrost, Connector as ConnectorConfig, UtxoRpc};
    use cardano_sdk::Network;

    #[test]
    fn blockfrost_config_without_project_id_is_not_runnable() {
        let config = ConnectorConfig::Blockfrost(Blockfrost {
            network: Network::Mainnet,
            project_id: None,
        });

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let error = match runtime.block_on(Connector::from_config(&config)) {
            Ok(_) => panic!("missing project id should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("KONDUIT_BLOCKFROST_PROJECT_ID"));
    }

    #[test]
    fn utxorpc_config_without_uri_is_not_runnable() {
        let config = ConnectorConfig::UtxoRpc(UtxoRpc {
            network: Network::Preview,
            uri: None,
        });

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let error = match runtime.block_on(Connector::from_config(&config)) {
            Ok(_) => panic!("missing URI should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("KONDUIT_UTXORPC_URI"));
    }

    #[test]
    fn blockfrost_network_mismatch_is_rejected_before_runtime_use() {
        let config = ConnectorConfig::Blockfrost(Blockfrost {
            network: Network::Preview,
            project_id: Some("preprod12345".to_string()),
        });

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let error = match runtime.block_on(Connector::from_config(&config)) {
            Ok(_) => panic!("network mismatch should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("does not match"));
    }

    #[test]
    fn blockfrost_validation_accepts_matching_project_id() {
        let config = Blockfrost {
            network: Network::Preview,
            project_id: Some("preview12345".to_string()),
        };

        let project_id = validated_blockfrost_project_id(&config)
            .expect("matching Blockfrost config should validate");

        assert_eq!(project_id, "preview12345");
    }
}
