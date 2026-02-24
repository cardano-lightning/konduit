use crate::{HttpClient, TransactionSummary, helpers::singleton};
use anyhow::anyhow;
use cardano_connect::{CardanoConnect, Network, NetworkName};
use cardano_tx_builder::{
    Address, Credential, Input, Output, ProtocolParameters, SigningKey, Transaction,
    VerificationKey,
    cbor::ToCbor,
    hash::Hash32,
    transaction::{TransactionReadyForSigning, state},
};
use std::{collections::BTreeMap, ops::Deref};
use wasm_bindgen::prelude::*;
use web_time::Duration;

mod balance;
mod health;
mod network;
mod utxos_at;

#[wasm_bindgen]
pub struct CardanoConnector {
    http_client: HttpClient,
    network: Network,
}

// -------------------------------------------------------------------- WASM API

#[wasm_bindgen]
impl CardanoConnector {
    #[wasm_bindgen]
    pub async fn new(base_url: &str, http_timeout_ms: Option<u64>) -> crate::Result<Self> {
        let http_client = HttpClient::new(
            base_url.strip_suffix("/").unwrap_or(base_url).to_string(),
            Duration::from_millis(http_timeout_ms.unwrap_or(10_000)),
        );

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

    #[wasm_bindgen(getter)]
    pub fn network(&self) -> NetworkName {
        NetworkName::from(self.network)
    }

    #[wasm_bindgen(js_name = "signAndSubmit")]
    pub async fn sign_and_submit(
        &self,
        transaction: &mut TransactionReadyForSigning,
        signing_key: &[u8],
    ) -> crate::Result<Hash32> {
        let signing_key: SigningKey = <[u8; 32]>::try_from(signing_key)
            .map_err(|_| anyhow!("invalid signing key length"))?
            .into();

        transaction.sign(&signing_key);

        let tx_hash = transaction.id();
        self.submit(transaction.deref()).await?;

        Ok(tx_hash.into())
    }

    // TODO: move 'balance' under the Connector trait.
    #[wasm_bindgen]
    pub async fn balance(&self, verification_key: &[u8]) -> crate::Result<u64> {
        let verification_key: VerificationKey = <[u8; 32]>::try_from(verification_key)
            .map_err(|_| anyhow!("invalid verification key length"))?
            .into();

        let addr = verification_key.to_address(self.network.into());

        let balance = self
            .http_client
            .get::<balance::Response>(&format!("/balance/{addr}"))
            .await?;

        Ok(balance.lovelace.parse::<u64>().map_err(|e| anyhow!(e))?)
    }

    #[wasm_bindgen]
    pub async fn transactions(
        &self,
        payment: &Credential,
    ) -> crate::Result<Vec<TransactionSummary>> {
        let addr = Address::new((*self.network()).into(), payment.clone());
        Ok(self
            .http_client
            .get(&format!("/transactions/{addr}"))
            .await?)
    }

    #[wasm_bindgen(js_name = "health")]
    pub async fn _wasm_health(&self) -> crate::Result<String> {
        Ok(self.health().await?)
    }
}

// -------------------------------------------------------- CardanoConnect Trait

impl CardanoConnect for CardanoConnector {
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
        let mut addr = Address::new(self.network.into(), payment.clone());
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
        let body = singleton("transaction", hex::encode(transaction.to_cbor()))?;

        self.http_client
            .post::<serde_json::Value>("/submit", body)
            .await?;

        Ok(())
    }
}
