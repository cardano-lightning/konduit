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

pub async fn live_network(endpoint: &str) -> anyhow::Result<Network> {
    let builder = ClientBuilder::new()
        .uri(endpoint)
        .map_err(|error| anyhow!(error))
        .with_context(|| format!("invalid UTxO RPC endpoint {endpoint}"))?;

    let mut query = builder.build::<CardanoQueryClient>().await;

    let response = query
        .inner
        .read_genesis(utxorpc::spec::query::ReadGenesisRequest { field_mask: None })
        .await
        .map_err(|error| anyhow!(error))
        .with_context(|| format!("failed to read Dolos genesis from {endpoint}"))?
        .into_inner();

    let genesis = match response.config {
        Some(utxorpc::spec::query::read_genesis_response::Config::Cardano(genesis)) => genesis,
        None => return Err(anyhow!("UTxO RPC returned no Cardano genesis config")),
    };

    network_from_genesis(&genesis)
        .with_context(|| format!("failed to derive live Cardano network from {endpoint}"))
}

pub fn ensure_network_matches(
    configured: Network,
    live: Network,
    endpoint: &str,
) -> anyhow::Result<()> {
    if configured == live {
        return Ok(());
    }

    Err(anyhow!(
        "configured Cardano network {configured} does not match live Dolos network {live} at {endpoint}"
    ))
}

fn network_from_genesis(genesis: &utxorpc::spec::cardano::Genesis) -> anyhow::Result<Network> {
    let network = match u64::from(genesis.network_magic) {
        Network::MAINNET_MAGIC => Network::Mainnet,
        Network::PREPROD_MAGIC => Network::Preprod,
        Network::PREVIEW_MAGIC => Network::Preview,
        other => {
            return Err(anyhow!(
                "unsupported Cardano network magic {other} in Dolos genesis"
            ));
        }
    };

    match (genesis.network_id.as_str(), network) {
        ("mainnet", Network::Mainnet)
        | ("testnet", Network::Preprod)
        | ("testnet", Network::Preview)
        | ("", _) => Ok(network),
        (network_id, derived) => Err(anyhow!(
            "Dolos genesis network_id {network_id} is inconsistent with network magic for {derived}"
        )),
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

#[cfg(test)]
mod tests {
    use super::{ensure_network_matches, network_from_genesis};
    use cardano_sdk::Network;

    fn genesis(network_id: &str, network_magic: u32) -> utxorpc::spec::cardano::Genesis {
        utxorpc::spec::cardano::Genesis {
            network_id: network_id.to_string(),
            network_magic,
            ..Default::default()
        }
    }

    #[test]
    fn network_from_genesis_maps_preview_magic() {
        let network = network_from_genesis(&genesis("testnet", Network::PREVIEW_MAGIC as u32))
            .expect("preview magic should map");

        assert_eq!(network, Network::Preview);
    }

    #[test]
    fn network_from_genesis_rejects_inconsistent_network_id() {
        let error = network_from_genesis(&genesis("mainnet", Network::PREPROD_MAGIC as u32))
            .expect_err("inconsistent genesis should fail");

        assert!(error.to_string().contains("inconsistent"));
    }

    #[test]
    fn ensure_network_matches_rejects_mismatch() {
        let error =
            ensure_network_matches(Network::Preview, Network::Preprod, "http://127.0.0.1:1337")
                .expect_err("network mismatch should fail");

        assert!(error.to_string().contains("does not match"));
    }
}
