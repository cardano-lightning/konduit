mod config;
mod mapping;
mod params;

pub use config::Config;

use anyhow::{Context, anyhow};
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Input, Network, Output, ProtocolParameters, Transaction, cbor::ToCbor,
    transaction::state,
};
use std::collections::BTreeMap;
use tokio::sync::Mutex;
use utxorpc::{CardanoQueryClient, CardanoSubmitClient, CardanoSyncClient, ClientBuilder};

const PAGE_SIZE: u32 = 100;

pub struct UtxoRpc {
    config: Config,
    query: Mutex<CardanoQueryClient>,
    submit: Mutex<CardanoSubmitClient>,
}

impl UtxoRpc {
    pub async fn connect(config: Config) -> anyhow::Result<Self> {
        let builder = ClientBuilder::new()
            .uri(config.endpoint())
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("invalid UTxO RPC endpoint {}", config.endpoint()))?;

        let query = builder.build::<CardanoQueryClient>().await;
        let submit = builder.build::<CardanoSubmitClient>().await;

        Ok(Self {
            config,
            query: Mutex::new(query),
            submit: Mutex::new(submit),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    async fn read_tip(&self) -> anyhow::Result<utxorpc::spec::sync::BlockRef> {
        let mut sync = ClientBuilder::new()
            .uri(self.config.endpoint())
            .map_err(|error| anyhow!(error))?
            .build::<CardanoSyncClient>()
            .await;

        sync.read_tip()
            .await
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("failed to read Dolos tip from {}", self.config.endpoint()))?
            .ok_or_else(|| anyhow!("Dolos returned no tip"))
    }

    async fn load_utxos(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let mut query = self.query.lock().await;
        let predicate =
            mapping::predicate_for_credentials(self.config.network(), payment, delegation);
        let mut start = None;
        let mut all = BTreeMap::new();

        loop {
            let page = query
                .search_utxos(predicate.clone(), start.clone(), PAGE_SIZE)
                .await
                .map_err(|error| anyhow!(error))
                .with_context(|| {
                    format!(
                        "failed to search UTxOs from {} on {}",
                        self.config.endpoint(),
                        self.config.network()
                    )
                })?;

            for utxo in page.items {
                let (input, output) = mapping::map_output(utxo)?;

                if delegation.is_some() || mapping::matches_payment(&output, payment) {
                    all.insert(input, output);
                }
            }

            if let Some(next) = page.next {
                start = Some(next);
            } else {
                return Ok(all);
            }
        }
    }
}

impl CardanoConnector for UtxoRpc {
    fn network(&self) -> Network {
        self.config.network()
    }

    async fn health(&self) -> anyhow::Result<String> {
        let tip = self.read_tip().await?;

        Ok(format!(
            "utxorpc endpoint={} network={} tip_slot={} tip_hash={}",
            self.config.endpoint(),
            self.config.network(),
            tip.slot,
            hex::encode(tip.hash)
        ))
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        let mut query = self.query.lock().await;
        params::read(&mut query).await
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        self.load_utxos(payment, delegation).await
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        let mut submit = self.submit.lock().await;
        let tx = transaction.to_cbor();

        submit
            .submit_tx(tx)
            .await
            .map_err(|error| anyhow!(error))
            .with_context(|| {
                format!(
                    "failed to submit transaction via {}",
                    self.config.endpoint()
                )
            })?;

        Ok(())
    }
}
