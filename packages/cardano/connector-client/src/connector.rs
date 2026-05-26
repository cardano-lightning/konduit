use crate::{endpoints, types::TransactionSummary};
use anyhow::anyhow;
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Address, Credential, Input, Network, NetworkId, Output, ProtocolParameters, Transaction,
    VerificationKey, cbor::ToCbor, transaction::state,
};
use http_client::HttpClient;
use std::collections::BTreeMap;

/// A facade to a remote Cardano connector, abstracted over an HttpClient.
pub struct Connector<Http: HttpClient> {
    http_client: Http,
    network: Network,
}

impl<Http: HttpClient> Connector<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    pub async fn new(http_client: Http) -> anyhow::Result<Self> {
        let network = Network::try_from(
            http_client
                .get::<endpoints::network::Response>("/network")
                .await
                .map_err(|e| anyhow!(e))?
                .network
                .as_str(),
        )?;

        Ok(Self {
            http_client,
            network,
        })
    }

    pub fn base_url(&self) -> &str {
        self.http_client.base_url()
    }

    pub async fn balance(&self, verification_key: VerificationKey) -> anyhow::Result<u64> {
        let addr = verification_key.to_address(NetworkId::from(self.network));

        let balance = self
            .http_client
            .get::<endpoints::balance::Response>(&format!("/balance/{addr}"))
            .await
            .map_err(|e| anyhow!(e))?;

        balance.lovelace.parse::<u64>().map_err(|e| anyhow!(e))
    }

    pub async fn transactions(
        &self,
        payment: &Credential,
    ) -> anyhow::Result<Vec<TransactionSummary>> {
        // FIXME: transactions & delegated addresses
        // This should also fetch transactions associated with delegated addresses.
        let addr = Address::new(NetworkId::from(self.network), payment.clone());
        self.http_client
            .get::<Vec<TransactionSummary>>(&format!("/transactions/{addr}"))
            .await
            .map_err(|e| anyhow!(e))
    }
}

// -------------------------------------------------------- CardanoConnector Trait

impl<Http: HttpClient> CardanoConnector for Connector<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    fn network(&self) -> Network {
        self.network
    }

    async fn health(&self) -> anyhow::Result<String> {
        let health = self
            .http_client
            .get::<endpoints::health::Response>("/health")
            .await
            .map_err(|e| anyhow!(e))?;
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
            .get::<Vec<endpoints::utxos_at::Response>>(&format!("/utxos_at/{addr}"))
            .await
            .map_err(|e| anyhow!(e))?;

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
                Http::to_json(&SubmitRequest {
                    transaction: hex::encode(transaction.to_cbor()),
                }),
            )
            .await
            .map_err(|e| anyhow!(e))?;

        Ok(())
    }
}
