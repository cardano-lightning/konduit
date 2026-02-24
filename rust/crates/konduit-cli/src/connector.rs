use anyhow::{Result, bail};
use cardano_connector_client::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction,
    transaction::state::ReadyForSigning,
};
use std::collections::BTreeMap;

#[cfg(feature = "blockfrost")]
use cardano_connector_client::blockfrost;

/// A wrapper enum that allows switching between different Cardano connection
/// implementations based on configuration and enabled features.
#[allow(dead_code)]
pub enum Connector {
    #[cfg(feature = "blockfrost")]
    Blockfrost(blockfrost::Connector),
    None,
}

impl Connector {
    pub fn new_blockfrost(project_id: &str) -> anyhow::Result<Connector> {
        #[cfg(feature = "blockfrost")]
        {
            Ok(Connector::Blockfrost(blockfrost::Connector::new(
                project_id.to_string(),
            )))
        }
        #[cfg(not(feature = "blockfrost"))]
        {
            panic!("blockfrost connector not available. Try including as a feature")
        }
    }
}

impl CardanoConnector for Connector {
    fn network(&self) -> Network {
        match self {
            #[cfg(feature = "blockfrost")]
            Self::Blockfrost(c) => c.network(),
            Self::None => panic!("No connector configured. Please check your features and config."),
        }
    }

    async fn health(&self) -> Result<String> {
        match self {
            #[cfg(feature = "blockfrost")]
            Self::Blockfrost(c) => c.health().await,
            Self::None => bail!("No connector available"),
        }
    }

    async fn protocol_parameters(&self) -> Result<ProtocolParameters> {
        match self {
            #[cfg(feature = "blockfrost")]
            Self::Blockfrost(c) => c.protocol_parameters().await,
            Self::None => bail!("No connector available"),
        }
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> Result<BTreeMap<Input, Output>> {
        match self {
            #[cfg(feature = "blockfrost")]
            Self::Blockfrost(c) => c.utxos_at(payment, delegation).await,
            Self::None => bail!("No connector available"),
        }
    }

    async fn submit(&self, transaction: &Transaction<ReadyForSigning>) -> Result<()> {
        match self {
            #[cfg(feature = "blockfrost")]
            Self::Blockfrost(c) => c.submit(transaction).await,
            Self::None => bail!("No connector available"),
        }
    }
}
