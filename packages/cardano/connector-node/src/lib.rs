mod config;
mod genesis_config;
mod mapping;
mod params;
mod v3_cost_model;

pub use config::Config;

use anyhow::{Context, anyhow};
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction, cbor::ToCbor,
    transaction::state,
};
use pallas_network::{
    facades::NodeClient,
    miniprotocols::{
        localstate,
        localstate::queries_v16,
        localtxsubmission::{EraTx, Response as SubmissionResponse},
    },
};
use std::collections::BTreeMap;
use tokio::sync::Mutex;

/// A [`CardanoConnector`] backed by a local Cardano node reachable over the
/// n2c Ouroboros miniprotocol suite (Unix socket on Unix-like systems, named
/// pipe on Windows).
///
/// This is intended for testing against a local testnet and avoids the need to
/// spin up additional services like Blockfrost or UTxO RPC.
pub struct Node {
    config: Config,
    inner: Mutex<NodeClient>,
}

impl Node {
    /// Connect to the local node and perform the n2c handshake. The handshake
    /// negotiates the protocol versions; on mismatch with the node the
    /// underlying pallas multiplexer returns an incompatible-version error.
    pub async fn connect(config: Config) -> anyhow::Result<Self> {
        let client = NodeClient::connect(config.socket(), config.network_magic())
            .await
            .map_err(|error| anyhow!("failed to connect to local node: {error}"))
            .with_context(|| {
                format!(
                    "failed to connect to local node at {} (network magic {})",
                    config.socket().display(),
                    config.network_magic()
                )
            })?;

        Ok(Self {
            config,
            inner: Mutex::new(client),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Acquire the local-state protocol against the node and run `op` with
    /// the acquired client. Once `op` returns (success or failure), the
    /// protocol is released back to the server. Any error during acquire or
    /// release surfaces from this method; errors from `op` are returned as-is.
    ///
    /// The lock guard lives across `op` so no other miniprotocol can use the
    /// multiplexer concurrently.
    async fn with_state_query<R>(
        &self,
        op: impl AsyncFnOnce(&mut localstate::Client) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let mut guard = self.inner.lock().await;
        let client = guard.statequery();
        client
            .acquire(None)
            .await
            .map_err(|error| anyhow!(error))
            .context("failed to acquire local-state query")?;

        let result = op(client).await;

        let _ = client.send_release().await;

        result
    }
}

impl CardanoConnector for Node {
    fn network(&self) -> Network {
        Network::new(self.config.network_magic())
    }

    async fn health(&self) -> anyhow::Result<String> {
        self.with_state_query(async |client| {
            let point = queries_v16::get_chain_point(client)
                .await
                .map_err(|error| anyhow!(error))
                .context("failed to read chain point from local node")?;

            Ok(format!(
                "local-node socket={} network_magic={} tip={:?}",
                self.config.socket().display(),
                self.config.network_magic(),
                point
            ))
        })
        .await
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        self.with_state_query(async move |client| {
            params::read(client)
                .await
                .map_err(|error| anyhow!(error))
                .context("failed to read protocol parameters from local node")
        })
        .await
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let addrs = mapping::addrs_for_query(payment, delegation)?;
        self.with_state_query(async move |client| {
            let era =
                pallas_network::miniprotocols::localstate::queries_v16::get_current_era(client)
                    .await
                    .map_err(|error| anyhow!(error))
                    .context("failed to query current era from local node")?;

            let utxos =
                pallas_network::miniprotocols::localstate::queries_v16::get_utxo_by_address(
                    client, era, addrs,
                )
                .await
                .map_err(|error| anyhow!(error))
                .with_context(|| format!("failed to query UTxOs by address for {payment:?}"))?;

            mapping::map_utxos(utxos)
        })
        .await
    }

    async fn submit(&self, tx: &Transaction<state::ReadyForSigning>) -> anyhow::Result<()> {
        let bytes = tx.to_cbor();
        let mut guard = self.inner.lock().await;

        let era = {
            let client = guard.statequery();
            client
                .acquire(None)
                .await
                .map_err(|error| anyhow!(error))
                .context("failed to acquire local-state query before submitting")?;
            let era =
                pallas_network::miniprotocols::localstate::queries_v16::get_current_era(client)
                    .await
                    .map_err(|error| anyhow!(error))
                    .context("failed to query current era before submitting transaction")?;
            let _ = client.send_release().await;
            era
        };

        let response = guard
            .submission()
            .submit_tx(EraTx(era, bytes))
            .await
            .map_err(|error| anyhow!(error))
            .context("failed to submit transaction via local node")?;

        match response {
            SubmissionResponse::Accepted => Ok(()),
            SubmissionResponse::Rejected(reason) => Err(anyhow!(
                "local node rejected transaction: 0x{}",
                hex::encode(&reason.0)
            )),
        }
    }
}
