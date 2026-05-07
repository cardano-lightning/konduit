mod args;
pub use args::CardanoArgs as Args;

use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction, transaction::state,
};
use std::collections::BTreeMap;

pub enum Cardano {
    Blockfrost(cardano_connector_direct::Blockfrost),
    UtxoRpc(Box<cardano_connector_utxorpc::UtxoRpc>),
}

impl CardanoConnector for Cardano {
    fn network(&self) -> Network {
        match self {
            Self::Blockfrost(connector) => connector.network(),
            Self::UtxoRpc(connector) => connector.network(),
        }
    }

    async fn health(&self) -> anyhow::Result<String> {
        match self {
            Self::Blockfrost(connector) => connector.health().await,
            Self::UtxoRpc(connector) => connector.health().await,
        }
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        match self {
            Self::Blockfrost(connector) => connector.protocol_parameters().await,
            Self::UtxoRpc(connector) => connector.protocol_parameters().await,
        }
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        match self {
            Self::Blockfrost(connector) => connector.utxos_at(payment, delegation).await,
            Self::UtxoRpc(connector) => connector.utxos_at(payment, delegation).await,
        }
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Blockfrost(connector) => connector.submit(transaction).await,
            Self::UtxoRpc(connector) => connector.submit(transaction).await,
        }
    }
}
