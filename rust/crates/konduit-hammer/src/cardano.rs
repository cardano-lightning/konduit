use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction, transaction::state,
};
use http_client_native::HttpClient;
use std::collections::BTreeMap;
use std::future::Future;

/// This antipattern exists because `CardanoConnector` does not dyn compatible.
pub enum Cardano {
    Blockfrost(cardano_connector_direct::Blockfrost),
    Client(cardano_connector_client::Connector<HttpClient>),
    // Placeholder for future implementations
    Todo(Dummy),
}

impl CardanoConnector for Cardano {
    fn network(&self) -> Network {
        match self {
            Self::Blockfrost(c) => c.network(),
            Self::Client(c) => c.network(),
            Self::Todo(c) => c.network(),
        }
    }

    fn health(&self) -> impl Future<Output = anyhow::Result<String>> {
        async move {
            match self {
                Self::Blockfrost(c) => c.health().await,
                Self::Client(c) => c.health().await,
                Self::Todo(c) => c.health().await,
            }
        }
    }

    fn protocol_parameters(&self) -> impl Future<Output = anyhow::Result<ProtocolParameters>> {
        async move {
            match self {
                Self::Blockfrost(c) => c.protocol_parameters().await,
                Self::Client(c) => c.protocol_parameters().await,
                Self::Todo(c) => c.protocol_parameters().await,
            }
        }
    }

    fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> impl Future<Output = anyhow::Result<BTreeMap<Input, Output>>> {
        // We clone or copy small references if needed for the async block
        async move {
            match self {
                Self::Blockfrost(c) => c.utxos_at(payment, delegation).await,
                Self::Client(c) => c.utxos_at(payment, delegation).await,
                Self::Todo(c) => c.utxos_at(payment, delegation).await,
            }
        }
    }

    fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = anyhow::Result<()>> {
        async move {
            match self {
                Self::Blockfrost(c) => c.submit(transaction).await,
                Self::Client(c) => c.submit(transaction).await,
                Self::Todo(c) => c.submit(transaction).await,
            }
        }
    }
}

/// A placeholder implementation of CardanoConnector where all methods are unimplemented.
pub struct Dummy;

impl CardanoConnector for Dummy {
    fn network(&self) -> Network {
        todo!("Dummy network not implemented")
    }

    fn health(&self) -> impl Future<Output = anyhow::Result<String>> {
        async move { todo!("Dummy health not implemented") }
    }

    fn protocol_parameters(&self) -> impl Future<Output = anyhow::Result<ProtocolParameters>> {
        async move { todo!("Dummy protocol_parameters not implemented") }
    }

    fn utxos_at(
        &self,
        _payment: &Credential,
        _delegation: Option<&Credential>,
    ) -> impl Future<Output = anyhow::Result<BTreeMap<Input, Output>>> {
        async move { todo!("Dummy utxos_at not implemented") }
    }

    fn submit(
        &self,
        _transaction: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = anyhow::Result<()>> {
        async move { todo!("Dummy submit not implemented") }
    }
}
