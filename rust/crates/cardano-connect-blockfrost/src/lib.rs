use anyhow::{Result, anyhow};
use futures::stream::{self, StreamExt};
use std::collections::BTreeMap;

use cardano_tx_builder::{
    Address, Credential, Input, Output, PlutusData, PlutusScript, PlutusVersion,
    ProtocolParameters, Value, address::kind, cbor,
};

use cardano_connect::{CardanoConnect, Network};

use blockfrost::{BlockfrostAPI, Pagination};
use blockfrost_openapi::models::{
    address_utxo_content_inner::AddressUtxoContentInner,
    tx_content_output_amount_inner::TxContentOutputAmountInner,
};

const UNIT_LOVELACE: &str = "lovelace";

const MAINNET_PREFIX: &str = "mainnet";
const PREPROD_PREFIX: &str = "preprod";
const PREVIEW_PREFIX: &str = "preview";

pub struct Blockfrost {
    api: BlockfrostAPI,
    base_url: String,
    client: reqwest::Client,
    network: Network,
    project_id: String,
}

impl Blockfrost {
    pub fn new(project_id: String) -> Self {
        let network_prefix = &project_id[0..7];
        let network = match network_prefix {
            MAINNET_PREFIX => Network::mainnet(),
            PREVIEW_PREFIX => Network::preview(),
            PREPROD_PREFIX => Network::preprod(),
            _ => panic!("Unknown network not yet supported"),
        };
        let base_url = format!("https://cardano-{}.blockfrost.io/api/v0", network_prefix,);
        let api = BlockfrostAPI::new(project_id.as_str(), Default::default());
        Self {
            api,
            base_url,
            client: reqwest::Client::new(),
            network,
            project_id,
        }
    }

    pub async fn resolve_datum_hash(&self, datum_hash: &str) -> Result<PlutusData> {
        let x = self.api.scripts_datum_hash_cbor(datum_hash).await?;
        let data = x
            .as_object()
            .ok_or(anyhow!("Expect an object"))?
            .get("cbor")
            .ok_or(anyhow!("Expect key `cbor`"))?
            .as_str()
            .ok_or(anyhow!("Expect value to be string"))?;
        Ok(cbor::decode(&hex::decode(&data)?)?)
    }

    pub async fn resolve_utxo(&self, bf_utxo: AddressUtxoContentInner) -> Result<(Input, Output)> {
        let input = Input::new(
            v2a(hex::decode(&bf_utxo.tx_hash)?)?.into(),
            bf_utxo.tx_index as u64,
        );
        let address = <Address<kind::Shelley>>::try_from(bf_utxo.address.as_str())?;
        let mut output = Output::new(
            address.into(),
            from_tx_content_output_amounts(&bf_utxo.amount[..])?,
        );
        if let Some(inline_datum) = &bf_utxo.inline_datum {
            output = output.with_datum(plutus_data_from_inline(&inline_datum)?);
        } else if let Some(datum_hash) = &bf_utxo.data_hash {
            let datum = self.resolve_datum_hash(datum_hash).await?;
            output = output.with_datum(datum);
        }
        if let Some(script_hash) = &bf_utxo.reference_script_hash {
            let script = self.resolve_script(&script_hash).await?;
            output = output.with_plutus_script(script);
        };
        Ok((input, output))
    }

    /// Blockfrost client has the wrong type.
    pub async fn resolve_script(&self, script_hash: &str) -> Result<PlutusScript> {
        let version = self.plutus_version(script_hash);
        let bytes = self.scripts_hash_cbor(script_hash);
        Ok(PlutusScript::new(version.await?, bytes.await?))
    }

    /// Blockfrost client has the wrong type.
    pub async fn scripts_hash_cbor(&self, script_hash: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(&format!("{}/scripts/{}", self.base_url, script_hash))
            .header("Accept", "application/json")
            .header("project_id", self.project_id.as_str())
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let ResponseCbor { cbor } = response.json::<ResponseCbor>().await.unwrap();
                let bytes = hex::decode(cbor)?;
                Ok(bytes)
            }
            _ => Err(anyhow!("No script found")),
        }
    }

    /// Blockfrost client has incomplete type
    pub async fn plutus_version(&self, script_hash: &str) -> Result<PlutusVersion> {
        let response = self
            .client
            .get(&format!("{}/scripts/{}/cbor", self.base_url, script_hash))
            .header("Accept", "application/json")
            .header("project_id", self.project_id.as_str())
            .send()
            .await
            .unwrap();

        match response.status() {
            reqwest::StatusCode::OK => {
                let ResponseScript { plutus_type, .. } =
                    response.json::<ResponseScript>().await.unwrap();
                match plutus_type.as_str() {
                    "plutusV1" => Ok(PlutusVersion::V1),
                    "plutusV2" => Ok(PlutusVersion::V2),
                    "plutusV3" => Ok(PlutusVersion::V3),
                    _ => Err(anyhow!("Unknown plutus version")),
                }
            }
            _ => Err(anyhow!("No script found")),
        }
    }
}

impl CardanoConnect for Blockfrost {
    fn network(&self) -> Network {
        self.network.clone()
    }

    async fn health(&self) -> Result<String> {
        match self.api.health().await {
            Ok(x) => Ok(format!("{:?}", x)),
            Err(y) => Err(anyhow!(y.to_string())),
        }
    }

    async fn protocol_parameters(&self) -> Result<ProtocolParameters> {
        // FIXME :: The api does not expose all the required values.
        // Until this is fixed use the precompiled values.
        // let x = self.api.epochs_latest_parameters().await?;
        // let pp = ProtocolParameters::default()
        //     .with_fee_per_byte(x.min_fee_a as u64)
        //     .with_fee_constant(x.min_fee_b as u64)
        //     .with_collateral_coefficient(
        //         x.collateral_percent
        //             .ok_or(anyhow!("Expect `collateral_percent`"))? as f64
        //             / 100.0,
        //     )
        //     .with_referenced_scripts_base_fee_per_byte(
        //         x.min_fee_ref_script_cost_per_byte
        //             .ok_or(anyhow!("Expect `min_fee_ref_script_cost_per_byte`"))?
        //             .round() as u64,
        //     )
        //     .with_referenced_scripts_fee_multiplier(Ratio::new(12, 10)) // Not in response
        //     .with_referenced_scripts_fee_step_size(25000) // Not in response
        //     .with_execution_price_mem(x.price_mem.ok_or(anyhow!("Expect `price_mem`"))?)
        //     .with_execution_price_cpu(x.price_step.ok_or(anyhow!("Expect `price_step`"))?)
        //     // FIXME :: Timeslots from mainnet
        //     .with_start_time(1506203091) // Not in response
        //     .with_first_shelley_slot(4492800) // Not in response
        //     .with_plutus_v3_cost_model(
        //         x.cost_models_raw
        //             .ok_or(anyhow!("Expect `cost_models_raw`"))?
        //             .ok_or(anyhow!("Expect `cost_models_raw`"))?
        //             .get("PlutusV3")
        //             .ok_or(anyhow!("Expect `cost_models_raw.PlutusV3`"))?
        //             .as_array()
        //             .ok_or(anyhow!("Expect array"))?
        //             .iter()
        //             .map(|x| {
        //                 x.as_number()
        //                     .ok_or(anyhow!("Expect Number"))
        //                     .and_then(|x| x.as_i64().ok_or(anyhow!("Expect i64")))
        //             })
        //             .collect::<Result<Vec<i64>>>()?,
        //     );
        let pp = match self.network() {
            Network::Mainnet => ProtocolParameters::mainnet(),
            Network::Preview => ProtocolParameters::preview(),
            Network::Preprod => ProtocolParameters::preprod(),
            Network::Other(_) => Err(anyhow!(
                "`ProtocolParameters` for network `Other` are unknown"
            ))?,
        };
        Ok(pp)
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: &Option<Credential>,
    ) -> Result<BTreeMap<Input, Output>> {
        let mut addr = Address::new(self.network().into(), payment.clone());
        if let Some(delegation) = delegation {
            addr = addr.with_delegation(delegation.clone());
        }
        let response = self
            .api
            .addresses_utxos(&format!("{}", addr), Pagination::all())
            .await?;
        let s = stream::iter(response)
            .map(move |bf_utxo| self.resolve_utxo(bf_utxo))
            .buffer_unordered(10)
            .collect::<Vec<Result<(Input, Output)>>>()
            .await;
        s.into_iter().collect::<Result<BTreeMap<Input, Output>>>()
    }

    async fn submit(&self, tx: Vec<u8>) -> Result<String> {
        Ok(self.api.transactions_submit(tx).await?)
    }
}

pub fn plutus_data_from_inline<'a>(inline_datum: &str) -> Result<PlutusData<'a>> {
    Ok(cbor::decode(&hex::decode(inline_datum)?)?)
}

/// Handles the map error
fn v2a<T, const N: usize>(v: Vec<T>) -> Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ResponseCbor {
    cbor: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ResponseScript {
    script_hash: String,
    #[serde(rename = "type")]
    plutus_type: String,
    serialised_size: u64,
}

fn from_tx_content_output_amounts(xs: &[TxContentOutputAmountInner]) -> Result<Value<u64>> {
    let mut v = Value::default();
    for asset in xs {
        let amount: u64 = asset.quantity.parse()?;
        if asset.unit == UNIT_LOVELACE {
            v.add(&Value::new(amount));
        } else {
            let hash: [u8; 28] = v2a(hex::decode(&asset.unit[0..56])?)?;
            let name: Vec<u8> = hex::decode(&asset.unit[56..])?;
            v.add(&Value::default().with_assets([(hash.into(), [(name, amount)])]));
        }
    }
    Ok(v)
}
