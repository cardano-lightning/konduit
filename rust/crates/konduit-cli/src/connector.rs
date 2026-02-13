use anyhow::{Result, bail};
use cardano_connect::{CardanoConnect, Network};
use cardano_tx_builder::transaction::state::ReadyForSigning;
use cardano_tx_builder::{Credential, Input, Output, ProtocolParameters, Transaction};
use std::collections::BTreeMap;

/// A wrapper enum that allows switching between different Cardano connection
/// implementations based on configuration and enabled features.
#[allow(dead_code)]
pub enum Connector {
    #[cfg(feature = "blockfrost")]
    Blockfrost(cardano_connect_blockfrost::Blockfrost),
    None,
}

impl Connector {
    pub fn new_blockfrost(project_id: &str) -> anyhow::Result<Connector> {
        #[cfg(feature = "blockfrost")]
        {
            Ok(Connector::Blockfrost(
                cardano_connect_blockfrost::Blockfrost::new(project_id.to_string()),
            ))
        }
        #[cfg(not(feature = "blockfrost"))]
        {
            panic!("blockfrost connector not available. Try including as a feature")
        }
    }
}

impl CardanoConnect for Connector {
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
