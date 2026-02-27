use crate::{
    CardanoConnector,
    types::{self, TransactionSummary},
};
use anyhow::anyhow;
use cardano_sdk::{
    Address, Credential, Input, Network, NetworkId, Output, ProtocolParameters, Transaction,
    VerificationKey, cbor::ToCbor, transaction::state, wasm,
};
use http_client::{HttpClient as _, wasm::HttpClient};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

mod balance;
mod health;
mod network;
mod utxos_at;

#[wasm_bindgen]
/// A facade to a remote Cardano connector.
pub struct Connector {
    http_client: HttpClient,
    network: Network,
}

// -------------------------------------------------------------------- WASM API

#[wasm_bindgen]
impl Connector {
    #[wasm_bindgen]
    pub async fn new(base_url: &str) -> wasm::Result<Self> {
        let http_client = HttpClient::new(base_url);

        let network = Network::try_from(
            http_client
                .get::<network::Response>("/network")
                .await?
                .network
                .as_str(),
        )?;

        Ok(Self {
            http_client,
            network,
        })
    }

    #[wasm_bindgen(getter, js_name = "networkId")]
    pub fn network_id(&self) -> wasm::NetworkId {
        NetworkId::from(self.network).into()
    }

    // TODO: move 'balance' under the Connector trait.
    #[wasm_bindgen(js_name = "balance")]
    pub async fn _wasm_balance(
        &self,
        verification_key: &wasm::VerificationKey,
    ) -> wasm::Result<u64> {
        let verification_key: VerificationKey = (*verification_key).into();

        let addr = verification_key.to_address(NetworkId::from(self.network));

        let balance = self
            .http_client
            .get::<balance::Response>(&format!("/balance/{addr}"))
            .await?;

        Ok(balance.lovelace.parse::<u64>().map_err(|e| anyhow!(e))?)
    }

    #[wasm_bindgen(js_name = "transactions")]
    pub async fn _wasm_transactions(
        &self,
        payment: &wasm::Credential,
    ) -> wasm::Result<Vec<types::wasm::TransactionSummary>> {
        let addr = Address::new(NetworkId::from(self.network), payment.clone().into());
        Ok(self
            .http_client
            .get::<Vec<TransactionSummary>>(&format!("/transactions/{addr}"))
            .await?
            .into_iter()
            .map(From::from)
            .collect())
    }

    #[wasm_bindgen(js_name = "health")]
    pub async fn _wasm_health(&self) -> wasm::Result<String> {
        Ok(self.health().await?)
    }
}

// -------------------------------------------------------- CardanoConnector Trait

impl CardanoConnector for Connector {
    fn network(&self) -> Network {
        self.network
    }

    async fn health(&self) -> anyhow::Result<String> {
        let health = self.http_client.get::<health::Response>("/health").await?;
        Ok(health.status)
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        Ok(self.network.into())
    }

    /// If delegation is None then it _should_ be ignored:
    /// Any address with matching payment credential should be returned.
    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let mut addr = Address::new(NetworkId::from(self.network), payment.clone());
        if let Some(delegation) = delegation {
            addr = addr.with_delegation(delegation.clone());
        }

        let utxos = self
            .http_client
            .get::<Vec<utxos_at::Response>>(&format!("/utxos_at/{addr}"))
            .await?;

        Ok(utxos
            .into_iter()
            .map(|obj| <(Input, Output)>::try_from(obj).expect("failed to convert UtxoObject"))
            .collect())
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        #[derive(serde::Serialize)]
        struct SubmitRequest {
            transaction: String,
        }

        self.http_client
            .post::<serde_json::Value>(
                "/submit",
                HttpClient::to_json(&SubmitRequest {
                    transaction: hex::encode(transaction.to_cbor()),
                }),
            )
            .await?;

        Ok(())
    }
}
