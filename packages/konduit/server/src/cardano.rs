mod args;
pub use args::CardanoArgs as Args;

use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Slot, Transaction, transaction::state,
};
use std::collections::BTreeMap;

pub enum Cardano {
    Blockfrost(cardano_connector_direct::Blockfrost),
    UtxoRpc(Box<cardano_connector_utxorpc::UtxoRpc>),
    Node(Box<cardano_connector_node::Node>),
}

impl CardanoConnector for Cardano {
    fn network(&self) -> Network {
        match self {
            Self::Blockfrost(connector) => connector.network(),
            Self::UtxoRpc(connector) => connector.network(),
            Self::Node(connector) => connector.network(),
        }
    }

    async fn health(&self) -> anyhow::Result<String> {
        match self {
            Self::Blockfrost(connector) => connector.health().await,
            Self::UtxoRpc(connector) => connector.health().await,
            Self::Node(connector) => connector.health().await,
        }
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        match self {
            Self::Blockfrost(connector) => connector.protocol_parameters().await,
            Self::UtxoRpc(connector) => connector.protocol_parameters().await,
            Self::Node(connector) => connector.protocol_parameters().await,
        }
    }

    async fn tip_slot(&self) -> anyhow::Result<Slot> {
        match self {
            Self::Blockfrost(connector) => connector.tip_slot().await,
            Self::UtxoRpc(connector) => connector.tip_slot().await,
            Self::Node(connector) => connector.tip_slot().await,
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
            Self::Node(connector) => connector.utxos_at(payment, delegation).await,
        }
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Blockfrost(connector) => connector.submit(transaction).await,
            Self::UtxoRpc(connector) => connector.submit(transaction).await,
            Self::Node(connector) => connector.submit(transaction).await,
        }
    }
}
