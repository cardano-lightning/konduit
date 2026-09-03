use crate::error::ApiError;
use crate::tx::{parse_lowercase_hex, parse_tx_id};
use crate::wire::{AssetObject, TransactionSummary, TxInput, TxOutput, Utxo};
use async_trait::async_trait;
use cardano_connector_utxorpc::{SubmitCbor, UtxoRpc};
use cardano_sdk::{Address, NetworkId, address::kind};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

const MAX_DOLOS_TIP_LAG: u64 = 20;

#[async_trait]
pub trait Ledger: Send + Sync {
    async fn ready(&self) -> Result<(), ApiError>;
    async fn tip(&self) -> Result<(u64, u64), ApiError>;
    async fn protocol_parameters(
        &self,
    ) -> Result<(String, u64, u64, cardano_connector_utxorpc::BloxbeanPayload), ApiError>;
    async fn utxos_at(&self, address: &Address<kind::Shelley>) -> Result<Vec<Utxo>, ApiError>;
    async fn read_tx(&self, txid: &[u8; 32]) -> Result<Option<u64>, ApiError>;
    async fn submit_cbor(&self, cbor: &[u8]) -> Result<SubmitCbor, ApiError>;
    async fn max_tx_size(&self) -> Result<u64, ApiError>;
}

#[async_trait]
pub trait History: Send + Sync {
    async fn ready(&self, dolos_height: u64) -> Result<(), ApiError>;
    async fn address_history(&self, address: &str) -> Result<Vec<TransactionSummary>, ApiError>;
    async fn transaction(&self, txid: &str) -> Result<Option<TransactionSummary>, ApiError>;
}

pub struct DolosLedger {
    inner: UtxoRpc,
}

impl DolosLedger {
    pub fn new(inner: UtxoRpc) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Ledger for DolosLedger {
    async fn ready(&self) -> Result<(), ApiError> {
        let start = self
            .inner
            .history_start_height()
            .await
            .map_err(ApiError::from)?;
        if start > 1 {
            return Err(ApiError::unavailable());
        }
        let _ = self
            .inner
            .bloxbean_parameters()
            .await
            .map_err(ApiError::from)?;
        Ok(())
    }

    async fn tip(&self) -> Result<(u64, u64), ApiError> {
        let tip = self.inner.tip().await.map_err(ApiError::from)?;
        Ok((tip.height, tip.slot))
    }

    async fn protocol_parameters(
        &self,
    ) -> Result<(String, u64, u64, cardano_connector_utxorpc::BloxbeanPayload), ApiError> {
        self.inner
            .bloxbean_parameters()
            .await
            .map_err(ApiError::from)
    }

    async fn utxos_at(&self, address: &Address<kind::Shelley>) -> Result<Vec<Utxo>, ApiError> {
        let bytes = Vec::from(address);
        Ok(self
            .inner
            .utxos_at_address(&bytes)
            .await
            .map_err(ApiError::from)?
            .into_iter()
            .map(Utxo::from)
            .collect())
    }

    async fn read_tx(&self, txid: &[u8; 32]) -> Result<Option<u64>, ApiError> {
        Ok(self
            .inner
            .read_tx(txid)
            .await
            .map_err(ApiError::from)?
            .and_then(|tx| tx.block_ref.map(|point| point.height)))
    }

    async fn submit_cbor(&self, cbor: &[u8]) -> Result<SubmitCbor, ApiError> {
        self.inner.submit_cbor(cbor).await.map_err(ApiError::from)
    }

    async fn max_tx_size(&self) -> Result<u64, ApiError> {
        let (_, _, _, payload) = self
            .inner
            .bloxbean_parameters()
            .await
            .map_err(ApiError::from)?;
        Ok(payload.max_tx_size)
    }
}

pub struct KoiosHistory {
    client: reqwest::Client,
    base: String,
}

impl KoiosHistory {
    pub fn new(base: String) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            base: base.trim_end_matches('/').to_string(),
        })
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let response = self
            .client
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .map_err(|_| ApiError::unavailable())?;
        if !response.status().is_success() {
            return Err(ApiError::unavailable());
        }
        response.json().await.map_err(|_| ApiError::unavailable())
    }
    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, ApiError> {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .json(&body)
            .send()
            .await
            .map_err(|_| ApiError::unavailable())?;
        let status = response.status();
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(ApiError::unavailable());
        }
        if !status.is_success() {
            return Err(ApiError::unavailable());
        }
        response.json().await.map_err(|_| ApiError::unavailable())
    }
}

#[async_trait]
impl History for KoiosHistory {
    async fn ready(&self, dolos_height: u64) -> Result<(), ApiError> {
        let checks: (
            Vec<KoiosGenesis>,
            Vec<KoiosTip>,
            Vec<AddressTx>,
            Vec<TxInfo>,
        ) = tokio::try_join!(
            self.get("/genesis"),
            self.get("/tip"),
            self.post(
                "/address_txs?limit=1",
                serde_json::json!({ "_addresses": [] })
            ),
            self.post(
                "/tx_info",
                serde_json::json!({
                    "_tx_hashes": [],
                    "_inputs": true,
                    "_assets": true,
                    "_scripts": true,
                    "_bytecode": true
                })
            )
        )?;
        let (genesis, tip, _, _) = checks;
        let magic = genesis
            .into_iter()
            .next()
            .map(|row| row.networkmagic)
            .ok_or_else(ApiError::unavailable)?;
        let koios_height = tip
            .into_iter()
            .next()
            .map(|row| row.block_height)
            .ok_or_else(ApiError::unavailable)?;
        if magic != 764824073 || koios_height.abs_diff(dolos_height) > MAX_DOLOS_TIP_LAG {
            return Err(ApiError::unavailable());
        }
        Ok(())
    }

    async fn address_history(&self, address: &str) -> Result<Vec<TransactionSummary>, ApiError> {
        let rows: Vec<AddressTx> = self
            .post(
                "/address_txs?order=block_height.desc&limit=100",
                serde_json::json!({ "_addresses": [address] }),
            )
            .await?;
        let mut hashes = Vec::new();
        for row in rows {
            if parse_tx_id(&row.tx_hash).is_ok() && !hashes.contains(&row.tx_hash) {
                hashes.push(row.tx_hash);
            }
            if hashes.len() == 100 {
                break;
            }
        }
        let mut out = Vec::new();
        for chunk in hashes.chunks(20) {
            let infos: Vec<TxInfo> = self
                .post(
                    "/tx_info",
                    serde_json::json!({
                        "_tx_hashes": chunk,
                        "_inputs": true,
                        "_assets": true,
                        "_scripts": true,
                        "_bytecode": true
                    }),
                )
                .await?;
            let mut hydrated = BTreeMap::new();
            for info in infos {
                let tx = map_tx(info)?;
                if !chunk.contains(&tx.id) || hydrated.insert(tx.id.clone(), tx).is_some() {
                    return Err(ApiError::unavailable());
                }
            }
            for hash in chunk {
                if let Some(tx) = hydrated.remove(hash)
                    && tx_touches(&tx, address)
                {
                    out.push(tx);
                }
            }
        }
        Ok(out)
    }

    async fn transaction(&self, txid: &str) -> Result<Option<TransactionSummary>, ApiError> {
        parse_tx_id(txid).map_err(|_| ApiError::bad_request())?;
        let infos: Vec<TxInfo> = self
            .post(
                "/tx_info",
                serde_json::json!({
                    "_tx_hashes": [txid],
                    "_inputs": true,
                    "_assets": true,
                    "_scripts": true,
                    "_bytecode": true
                }),
            )
            .await?;
        if infos.len() > 1 {
            return Err(ApiError::unavailable());
        }
        let Some(info) = infos.into_iter().next() else {
            return Ok(None);
        };
        if info.tx_hash != txid {
            return Err(ApiError::unavailable());
        }
        map_tx(info).map(Some)
    }
}

#[derive(Deserialize)]
struct KoiosGenesis {
    #[serde(deserialize_with = "deserialize_u64")]
    networkmagic: u64,
}

#[derive(Deserialize)]
struct KoiosTip {
    block_height: u64,
}

#[derive(Deserialize)]
struct AddressTx {
    tx_hash: String,
}

#[derive(Deserialize)]
struct TxInfo {
    tx_hash: String,
    block_height: u64,
    tx_timestamp: u64,
    tx_block_index: u64,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    invalid_before: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    invalid_after: Option<u64>,
    valid_contract: bool,
    inputs: Vec<KoiosIo>,
    outputs: Vec<KoiosIo>,
    #[serde(default, deserialize_with = "deserialize_null_vec")]
    collateral_inputs: Vec<KoiosIo>,
    #[serde(default, deserialize_with = "deserialize_null_vec")]
    collateral_output: Vec<KoiosIo>,
    #[serde(
        default,
        rename = "reference_inputs",
        deserialize_with = "deserialize_null_vec"
    )]
    _reference_inputs: Vec<KoiosIo>,
}

#[derive(Deserialize, Clone)]
struct KoiosIo {
    #[serde(default)]
    tx_hash: Option<String>,
    #[serde(default)]
    tx_index: Option<u32>,
    value: String,
    #[serde(deserialize_with = "deserialize_assets")]
    asset_list: Vec<KoiosAsset>,
    payment_addr: PaymentAddr,
    #[serde(default)]
    datum_hash: Option<String>,
    #[serde(default)]
    inline_datum: Option<InlineDatum>,
    #[serde(default)]
    reference_script: Option<RefScript>,
    #[serde(default)]
    collateral: bool,
    #[serde(default)]
    reference: bool,
}

#[derive(Deserialize, Clone)]
struct PaymentAddr {
    bech32: String,
}

#[derive(Deserialize, Clone)]
struct KoiosAsset {
    policy_id: String,
    #[serde(default)]
    asset_name: Option<String>,
    quantity: String,
}

fn deserialize_assets<'de, D>(deserializer: D) -> Result<Vec<KoiosAsset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Assets {
        List(Vec<KoiosAsset>),
        Json(String),
    }

    match Assets::deserialize(deserializer)? {
        Assets::List(assets) => Ok(assets),
        Assets::Json(json) => serde_json::from_str(&json).map_err(serde::de::Error::custom),
    }
}

#[derive(Deserialize, Clone)]
struct InlineDatum {
    bytes: String,
}

#[derive(Deserialize, Clone)]
struct RefScript {
    hash: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonU64 {
    Number(u64),
    String(String),
}

impl JsonU64 {
    fn get<E: serde::de::Error>(self) -> Result<u64, E> {
        match self {
            Self::Number(value) => Ok(value),
            Self::String(value) => value.parse().map_err(E::custom),
        }
    }
}

fn deserialize_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    JsonU64::deserialize(deserializer)?.get()
}

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<JsonU64>::deserialize(deserializer)?
        .map(JsonU64::get)
        .transpose()
}

fn deserialize_null_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

fn map_tx(info: TxInfo) -> Result<TransactionSummary, ApiError> {
    parse_tx_id(&info.tx_hash).map_err(|_| ApiError::unavailable())?;
    let inputs = if info.valid_contract {
        info.inputs
            .into_iter()
            .filter(|io| !io.reference && !io.collateral)
            .collect::<Vec<_>>()
    } else if !info.collateral_inputs.is_empty() {
        info.collateral_inputs
    } else {
        info.inputs.into_iter().filter(|io| io.collateral).collect()
    };
    if info.collateral_output.len() > 1 {
        return Err(ApiError::unavailable());
    }
    let outputs = if info.valid_contract {
        info.outputs
            .into_iter()
            .filter(|io| !io.collateral)
            .collect()
    } else if !info.collateral_output.is_empty() {
        info.collateral_output
    } else {
        info.outputs
            .into_iter()
            .filter(|io| io.collateral)
            .collect()
    };
    Ok(TransactionSummary {
        id: info.tx_hash,
        index: info.tx_block_index,
        depth: 0,
        block_height: info.block_height,
        timestamp: info.tx_timestamp,
        invalid_before: info.invalid_before,
        invalid_after: info.invalid_after,
        inputs: inputs
            .into_iter()
            .map(map_input)
            .collect::<Result<Vec<_>, _>>()?,
        outputs: outputs
            .into_iter()
            .map(map_output)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn map_input(io: KoiosIo) -> Result<TxInput, ApiError> {
    let transaction_id = io.tx_hash.clone().ok_or_else(ApiError::unavailable)?;
    parse_tx_id(&transaction_id).map_err(|_| ApiError::unavailable())?;
    let output_index = io.tx_index.ok_or_else(ApiError::unavailable)?;
    let output = map_output(io)?;
    Ok(TxInput {
        transaction_id,
        output_index,
        address: output.address,
        consumed_by: None,
        value: output.value,
        datum_hash: output.datum_hash,
        datum_inline: output.datum_inline,
        reference_script_hash: output.reference_script_hash,
    })
}

fn map_output(io: KoiosIo) -> Result<TxOutput, ApiError> {
    let address = io.payment_addr.bech32;
    let parsed =
        Address::<kind::Any>::try_from(address.as_str()).map_err(|_| ApiError::unavailable())?;
    if parsed
        .as_shelley()
        .is_none_or(|address| address.network_id() != NetworkId::MAINNET)
    {
        return Err(ApiError::unavailable());
    }
    let lovelace = io
        .value
        .parse::<u64>()
        .map_err(|_| ApiError::unavailable())?;
    let mut units = BTreeSet::from(["lovelace".to_string()]);
    let mut value = vec![AssetObject {
        unit: "lovelace".into(),
        quantity: lovelace.to_string(),
    }];
    for asset in io.asset_list {
        let policy = parse_lowercase_hex(&asset.policy_id).map_err(|_| ApiError::unavailable())?;
        let asset_name = asset.asset_name.unwrap_or_default();
        let name = if asset_name.is_empty() {
            Vec::new()
        } else {
            parse_lowercase_hex(&asset_name).map_err(|_| ApiError::unavailable())?
        };
        if policy.len() != 28 || name.len() > 32 {
            return Err(ApiError::unavailable());
        }
        asset
            .quantity
            .parse::<u64>()
            .map_err(|_| ApiError::unavailable())?;
        let unit = format!("{}{}", asset.policy_id, asset_name);
        if !units.insert(unit.clone()) {
            return Err(ApiError::unavailable());
        }
        value.push(AssetObject {
            unit,
            quantity: asset.quantity,
        });
    }
    let datum_hash = validate_hash(io.datum_hash, 32)?;
    let datum_inline = validate_hex(io.inline_datum.map(|datum| datum.bytes), 64 * 1024)?;
    let reference_script_hash = validate_hash(io.reference_script.map(|script| script.hash), 28)?;
    Ok(TxOutput {
        address,
        consumed_by: None,
        value,
        datum_hash,
        datum_inline,
        reference_script_hash,
    })
}

fn validate_hash(value: Option<String>, len: usize) -> Result<Option<String>, ApiError> {
    value
        .map(|hash| {
            let bytes = parse_lowercase_hex(&hash).map_err(|_| ApiError::unavailable())?;
            (bytes.len() == len)
                .then_some(hash)
                .ok_or_else(ApiError::unavailable)
        })
        .transpose()
}

fn validate_hex(value: Option<String>, max_bytes: usize) -> Result<Option<String>, ApiError> {
    value
        .map(|hex| {
            let bytes = parse_lowercase_hex(&hex).map_err(|_| ApiError::unavailable())?;
            (bytes.len() <= max_bytes)
                .then_some(hex)
                .ok_or_else(ApiError::unavailable)
        })
        .transpose()
}

fn tx_touches(tx: &TransactionSummary, address: &str) -> bool {
    tx.inputs.iter().any(|input| input.address == address)
        || tx.outputs.iter().any(|output| output.address == address)
}

pub fn parse_mainnet_address(raw: &str) -> Result<Address<kind::Shelley>, ApiError> {
    let address = Address::<kind::Shelley>::try_from(raw).map_err(|_| ApiError::bad_request())?;
    if address.network_id() != NetworkId::MAINNET {
        return Err(ApiError::bad_request());
    }
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koios_genesis_accepts_string_network_magic() {
        let rows: Vec<KoiosGenesis> =
            serde_json::from_str(r#"[{"networkmagic":"764824073"}]"#).unwrap();
        assert_eq!(rows.into_iter().next().unwrap().networkmagic, 764824073);
    }

    #[test]
    fn koios_collateral_accepts_json_encoded_assets() {
        let io: KoiosIo = serde_json::from_str(
            r#"{"value":"0","asset_list":"[]","payment_addr":{"bech32":"addr_test1..."}}"#,
        )
        .unwrap();
        assert!(io.asset_list.is_empty());
    }

    #[test]
    fn koios_transaction_rejects_missing_required_fields() {
        let result = serde_json::from_value::<TxInfo>(serde_json::json!({
            "tx_hash": "ab".repeat(32),
            "tx_timestamp": 1,
            "tx_block_index": 0,
            "valid_contract": true,
            "inputs": [],
            "outputs": []
        }));
        assert!(result.is_err());
    }

    #[test]
    fn hash_lengths_match_cardano_sizes() {
        let hash28 = "ab".repeat(28);
        let hash32 = "cd".repeat(32);
        assert!(validate_hash(Some(hash28.clone()), 28).unwrap().is_some());
        assert!(validate_hash(Some(hash32.clone()), 32).unwrap().is_some());
        assert!(validate_hash(Some(hash28), 32).is_err());
        assert!(validate_hash(Some(hash32), 28).is_err());
    }
}
