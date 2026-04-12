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
use std::future::Future;
use std::pin::Pin;
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
        collect_utxos_pages(payment, delegation, &mut *query, |query, start| {
            let predicate = predicate.clone();
            let endpoint = self.config.endpoint().to_owned();
            let network = self.config.network();
            Box::pin(async move {
                query
                    .search_utxos(predicate, start, PAGE_SIZE)
                    .await
                    .map_err(|error| anyhow!(error))
                    .with_context(|| format!("failed to search UTxOs from {endpoint} on {network}"))
            })
        })
        .await
    }
}

async fn collect_utxos_pages<S, F>(
    payment: &Credential,
    delegation: Option<&Credential>,
    state: &mut S,
    mut fetch_page: F,
) -> anyhow::Result<BTreeMap<Input, Output>>
where
    F: for<'a> FnMut(
        &'a mut S,
        Option<String>,
    ) -> Pin<
        Box<dyn Future<Output = anyhow::Result<utxorpc::UtxoPage<utxorpc::Cardano>>> + Send + 'a>,
    >,
{
    let mut start = None;
    let mut all = BTreeMap::new();

    loop {
        let page = fetch_page(state, start.clone()).await?;

        start = next_start_token(&page);
        let done = accumulate_page(&mut all, page, payment, delegation)?;
        if done {
            return Ok(all);
        }
    }
}

fn accumulate_page(
    all: &mut BTreeMap<Input, Output>,
    page: utxorpc::UtxoPage<utxorpc::Cardano>,
    payment: &Credential,
    delegation: Option<&Credential>,
) -> anyhow::Result<bool> {
    for utxo in page.items {
        let (input, output) = mapping::map_output(utxo)?;

        if delegation.is_some() || mapping::matches_payment(&output, payment) {
            all.insert(input, output);
        }
    }

    Ok(page.next.is_none())
}

fn next_start_token(page: &utxorpc::UtxoPage<utxorpc::Cardano>) -> Option<String> {
    page.next.clone()
}

fn submit_error(endpoint: &str, error: utxorpc::Error) -> anyhow::Error {
    anyhow!(error).context(format!("failed to submit transaction via {endpoint}"))
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
            .map_err(|error| submit_error(self.config.endpoint(), error))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_utxos_pages, ensure_network_matches, network_from_genesis, next_start_token,
        submit_error,
    };
    use cardano_sdk::{
        Datum, Network, Output, PlutusData, Value, address_test, cbor::ToCbor, key_credential,
    };
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use utxorpc::{ChainUtxo, Error as UtxoRpcError, NativeBytes, UtxoPage, spec};

    fn genesis(network_id: &str, network_magic: u32) -> utxorpc::spec::cardano::Genesis {
        utxorpc::spec::cardano::Genesis {
            network_id: network_id.to_string(),
            network_magic,
            ..Default::default()
        }
    }

    fn txo_ref(index: u32) -> spec::query::TxoRef {
        spec::query::TxoRef {
            hash: vec![index as u8; 32].into(),
            index,
        }
    }

    fn parsed_utxo(index: u32, output: Output) -> ChainUtxo<spec::cardano::TxOutput> {
        ChainUtxo {
            parsed: Some(spec::cardano::TxOutput {
                address: Vec::<u8>::from(output.address()).into(),
                coin: Some(spec::cardano::BigInt {
                    big_int: Some(spec::cardano::big_int::BigInt::Int(
                        output.value().lovelace() as i64,
                    )),
                }),
                assets: output
                    .value()
                    .assets()
                    .iter()
                    .map(|(policy, assets)| spec::cardano::Multiasset {
                        policy_id: Vec::from(policy.as_ref()).into(),
                        assets: assets
                            .iter()
                            .map(|(name, amount)| spec::cardano::Asset {
                                name: name.clone().into(),
                                quantity: Some(spec::cardano::asset::Quantity::OutputCoin(
                                    spec::cardano::BigInt {
                                        big_int: Some(spec::cardano::big_int::BigInt::Int(
                                            *amount as i64,
                                        )),
                                    },
                                )),
                            })
                            .collect(),
                        redeemer: None,
                    })
                    .collect(),
                datum: output.datum().map(|datum| match datum {
                    Datum::Hash(hash) => spec::cardano::Datum {
                        hash: Vec::from(hash.as_ref()).into(),
                        payload: None,
                        original_cbor: Default::default(),
                    },
                    Datum::Inline(data) => spec::cardano::Datum {
                        hash: Default::default(),
                        payload: None,
                        original_cbor: data.to_cbor().into(),
                    },
                }),
                script: None,
            }),
            native: NativeBytes::new(),
            txo_ref: Some(txo_ref(index)),
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
    fn network_from_genesis_allows_empty_network_id() {
        let network = network_from_genesis(&genesis("", Network::PREPROD_MAGIC as u32))
            .expect("empty network id should still map");

        assert_eq!(network, Network::Preprod);
    }

    #[test]
    fn network_from_genesis_rejects_unsupported_magic() {
        let error =
            network_from_genesis(&genesis("testnet", 999)).expect_err("unknown magic should fail");

        assert!(
            error
                .to_string()
                .contains("unsupported Cardano network magic")
        );
    }

    #[test]
    fn ensure_network_matches_rejects_mismatch() {
        let error =
            ensure_network_matches(Network::Preview, Network::Preprod, "http://127.0.0.1:1337")
                .expect_err("network mismatch should fail");

        assert!(error.to_string().contains("does not match"));
    }

    #[tokio::test]
    async fn collect_utxos_pages_filters_by_payment_across_pages() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let delegation =
            key_credential!("22222222222222222222222222222222222222222222222222222222");
        let other = key_credential!("33333333333333333333333333333333333333333333333333333333");
        let matched = Output::new(
            address_test!(payment.clone(), delegation.clone()).into(),
            Value::new(1_000_000),
        )
        .with_datum(PlutusData::integer(42));
        let unmatched = Output::new(address_test!(other).into(), Value::new(2_000_000));
        let second_page = Output::new(address_test!(payment.clone()).into(), Value::new(3_000_000));
        let mut pages = VecDeque::from([
            Ok(UtxoPage {
                items: vec![parsed_utxo(0, matched.clone()), parsed_utxo(1, unmatched)],
                next: Some("page-2".to_string()),
            }),
            Ok(UtxoPage {
                items: vec![parsed_utxo(2, second_page.clone())],
                next: None,
            }),
        ]);
        let starts = Arc::new(Mutex::new(Vec::new()));

        let all = collect_utxos_pages(&payment, None, &mut pages, {
            let starts = Arc::clone(&starts);
            move |pages, start| {
                let starts = Arc::clone(&starts);
                Box::pin(async move {
                    starts.lock().expect("starts lock").push(start);
                    pages.pop_front().expect("page")
                })
            }
        })
        .await
        .expect("pages should accumulate");

        assert_eq!(
            starts.lock().expect("starts lock").as_slice(),
            &[None, Some("page-2".to_string())]
        );
        assert_eq!(all.len(), 2);
        assert!(all.values().any(|output| output == &matched));
        assert!(all.values().any(|output| output == &second_page));
    }

    #[tokio::test]
    async fn collect_utxos_pages_propagates_mapping_errors() {
        let payment = key_credential!("11111111111111111111111111111111111111111111111111111111");
        let mut pages = vec![Ok(UtxoPage {
            items: vec![ChainUtxo {
                parsed: None,
                native: NativeBytes::new(),
                txo_ref: Some(txo_ref(0)),
            }],
            next: None,
        })]
        .into_iter();

        let error = collect_utxos_pages(&payment, None, &mut pages, |pages, _start| {
            Box::pin(async move { pages.next().expect("page") })
        })
        .await
        .expect_err("invalid page item should fail");

        assert!(
            format!("{error:#}").contains("UTxO response missing parsed output and native bytes")
        );
    }

    #[test]
    fn next_start_token_preserves_pagination_cursor() {
        let token = next_start_token(&UtxoPage::<utxorpc::Cardano> {
            items: Vec::new(),
            next: Some("cursor-2".to_string()),
        });

        assert_eq!(token.as_deref(), Some("cursor-2"));
    }

    #[test]
    fn submit_error_wraps_upstream_status_with_endpoint_context() {
        let error = submit_error(
            "http://127.0.0.1:1337",
            UtxoRpcError::ParseError("bad tx".to_string()),
        );

        let rendered = format!("{error:#}");
        assert!(rendered.contains("failed to submit transaction via http://127.0.0.1:1337"));
        assert!(rendered.contains("parse error"));
    }
}
