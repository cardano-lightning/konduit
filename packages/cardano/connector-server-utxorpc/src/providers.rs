use crate::error::ApiError;
use crate::tx::{parse_lowercase_hex, parse_tx_id};
use crate::wire::{AssetObject, TransactionSummary, TxInput, TxOutput, Utxo};
use async_trait::async_trait;
use cardano_connector_utxorpc::{SubmitCbor, UtxoRpc};
use cardano_sdk::{Address, NetworkId, address::kind};
use serde::Deserialize;
use std::{collections::BTreeSet, time::Duration};

#[derive(Debug, Clone)]
pub struct TxPresence {
    pub height: u64,
}

#[derive(Debug, Clone)]
pub enum SubmitResult {
    Accepted([u8; 32]),
    AlreadyKnown,
    InputsSpent,
    Indeterminate,
    Rejected,
}

#[async_trait]
pub trait Ledger: Send + Sync {
    async fn ready(&self) -> Result<(), ApiError>;
    async fn tip(&self) -> Result<(u64, u64), ApiError>;
    async fn protocol_parameters(
        &self,
    ) -> Result<(String, u64, u64, cardano_connector_utxorpc::BloxbeanPayload), ApiError>;
    async fn utxos_at(&self, address: &Address<kind::Shelley>) -> Result<Vec<Utxo>, ApiError>;
    async fn read_tx(&self, txid: &[u8; 32]) -> Result<Option<TxPresence>, ApiError>;
    async fn submit_cbor(&self, cbor: &[u8]) -> Result<SubmitResult, ApiError>;
    async fn max_tx_size(&self) -> Result<u64, ApiError>;
}

#[async_trait]
pub trait History: Send + Sync {
    async fn ping(&self) -> Result<(), ApiError>;
    async fn address_history(
        &self,
        address: &str,
        tip_height: u64,
    ) -> Result<Vec<TransactionSummary>, ApiError>;
    async fn transaction(
        &self,
        txid: &str,
        tip_height: u64,
    ) -> Result<Option<TransactionSummary>, ApiError>;
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
        let _ = self.inner.tip().await.map_err(ApiError::from)?;
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

    async fn read_tx(&self, txid: &[u8; 32]) -> Result<Option<TxPresence>, ApiError> {
        Ok(self
            .inner
            .read_tx(txid)
            .await
            .map_err(ApiError::from)?
            .and_then(|tx| {
                tx.block_ref.map(|point| TxPresence {
                    height: point.height,
                })
            }))
    }

    async fn submit_cbor(&self, cbor: &[u8]) -> Result<SubmitResult, ApiError> {
        Ok(
            match self.inner.submit_cbor(cbor).await.map_err(ApiError::from)? {
                SubmitCbor::Accepted(hash) => SubmitResult::Accepted(hash),
                SubmitCbor::AlreadyKnown => SubmitResult::AlreadyKnown,
                SubmitCbor::InputsSpent => SubmitResult::InputsSpent,
                SubmitCbor::Indeterminate => SubmitResult::Indeterminate,
                SubmitCbor::Rejected => SubmitResult::Rejected,
            },
        )
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
    async fn ping(&self) -> Result<(), ApiError> {
        let response = self
            .client
            .get(format!("{}/genesis", self.base))
            .send()
            .await
            .map_err(|_| ApiError::unavailable())?;
        if !response.status().is_success() {
            return Err(ApiError::unavailable());
        }
        let genesis: Vec<KoiosGenesis> =
            response.json().await.map_err(|_| ApiError::unavailable())?;
        let magic = genesis
            .into_iter()
            .next()
            .and_then(|row| json_u64(row.networkmagic))
            .ok_or_else(ApiError::unavailable)?;
        if magic != 764824073 {
            return Err(ApiError::unavailable());
        }
        Ok(())
    }

    async fn address_history(
        &self,
        address: &str,
        tip_height: u64,
    ) -> Result<Vec<TransactionSummary>, ApiError> {
        tokio::time::timeout(Duration::from_secs(19), async {
            let rows: Vec<AddressTx> = self
                .post(
                    "/address_txs?limit=100",
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
                for info in infos {
                    if let Some(tx) = map_tx(info, tip_height)?
                        && tx_touches(&tx, address)
                    {
                        out.push(tx);
                    }
                }
            }
            if out.len() > 1000 {
                out.truncate(1000);
            }
            Ok(out)
        })
        .await
        .map_err(|_| ApiError::unavailable())?
    }

    async fn transaction(
        &self,
        txid: &str,
        tip_height: u64,
    ) -> Result<Option<TransactionSummary>, ApiError> {
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
        let Some(info) = infos.into_iter().next() else {
            return Ok(None);
        };
        if info.tx_hash != txid {
            return Err(ApiError::unavailable());
        }
        map_tx(info, tip_height)
    }
}

#[derive(Deserialize)]
struct KoiosGenesis {
    #[serde(default)]
    networkmagic: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct AddressTx {
    tx_hash: String,
}

#[derive(Deserialize)]
struct TxInfo {
    tx_hash: String,
    #[serde(default)]
    block_height: Option<u64>,
    #[serde(default)]
    tx_timestamp: Option<u64>,
    #[serde(default)]
    tx_block_index: Option<u64>,
    #[serde(default)]
    invalid_before: Option<serde_json::Value>,
    #[serde(default)]
    invalid_after: Option<serde_json::Value>,
    #[serde(default)]
    valid_contract: Option<bool>,
    #[serde(default)]
    inputs: Vec<KoiosIo>,
    #[serde(default)]
    outputs: Vec<KoiosIo>,
    #[serde(default)]
    collateral_inputs: Vec<KoiosIo>,
    #[serde(default)]
    collateral_output: Option<KoiosIo>,
    #[serde(default)]
    reference_inputs: Vec<KoiosIo>,
}

#[derive(Deserialize, Clone)]
struct KoiosIo {
    #[serde(default)]
    tx_hash: Option<String>,
    #[serde(default)]
    tx_index: Option<u32>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default, deserialize_with = "deserialize_assets")]
    asset_list: Vec<KoiosAsset>,
    payment_addr: Option<PaymentAddr>,
    #[serde(default)]
    datum_hash: Option<String>,
    #[serde(default)]
    inline_datum: Option<InlineDatum>,
    #[serde(default)]
    reference_script: Option<RefScript>,
    #[serde(default)]
    collateral: Option<bool>,
    #[serde(default)]
    reference: Option<bool>,
}

#[derive(Deserialize, Clone)]
struct PaymentAddr {
    #[serde(default)]
    bech32: Option<String>,
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
    #[serde(default)]
    bytes: Option<String>,
}

#[derive(Deserialize, Clone)]
struct RefScript {
    #[serde(default)]
    hash: Option<String>,
}

fn json_u64(value: Option<serde_json::Value>) -> Option<u64> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn map_tx(info: TxInfo, tip_height: u64) -> Result<Option<TransactionSummary>, ApiError> {
    parse_tx_id(&info.tx_hash).map_err(|_| ApiError::unavailable())?;
    let success = info.valid_contract.unwrap_or(true);
    let inputs = if success {
        info.inputs
            .into_iter()
            .filter(|io| !io.reference.unwrap_or(false) && !io.collateral.unwrap_or(false))
            .chain(
                // Koios may split collateral/reference rather than flag them
                Vec::new(),
            )
            .collect::<Vec<_>>()
    } else if !info.collateral_inputs.is_empty() {
        info.collateral_inputs
    } else {
        info.inputs
            .into_iter()
            .filter(|io| io.collateral.unwrap_or(false))
            .collect()
    };
    let _ = info.reference_inputs;
    let outputs = if success {
        info.outputs
            .into_iter()
            .filter(|io| !io.collateral.unwrap_or(false))
            .collect()
    } else if let Some(output) = info.collateral_output {
        vec![output]
    } else {
        info.outputs
            .into_iter()
            .filter(|io| io.collateral.unwrap_or(false))
            .collect()
    };
    let height = info.block_height.unwrap_or(0);
    let depth = if height > tip_height {
        log::warn!("provider height ahead of Dolos tip");
        0
    } else {
        tip_height - height
    };
    Ok(Some(TransactionSummary {
        id: info.tx_hash,
        index: info.tx_block_index.unwrap_or(0),
        depth,
        timestamp: info.tx_timestamp.unwrap_or(0),
        invalid_before: json_u64(info.invalid_before),
        invalid_after: json_u64(info.invalid_after),
        inputs: inputs
            .into_iter()
            .map(map_input)
            .collect::<Result<Vec<_>, _>>()?,
        outputs: outputs
            .into_iter()
            .map(map_output)
            .collect::<Result<Vec<_>, _>>()?,
    }))
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
    let address = io
        .payment_addr
        .and_then(|addr| addr.bech32)
        .ok_or_else(ApiError::unavailable)?;
    Address::<kind::Any>::try_from(address.as_str()).map_err(|_| ApiError::unavailable())?;
    let mut units = BTreeSet::new();
    let mut value = Vec::new();
    if let Some(lovelace) = io.value {
        lovelace
            .parse::<u64>()
            .map_err(|_| ApiError::unavailable())?;
        units.insert("lovelace".to_string());
        value.push(AssetObject {
            unit: "lovelace".into(),
            quantity: lovelace,
        });
    }
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
    let datum_inline = validate_hex(io.inline_datum.and_then(|datum| datum.bytes))?;
    let reference_script_hash =
        validate_hash(io.reference_script.and_then(|script| script.hash), 28)?;
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

fn validate_hex(value: Option<String>) -> Result<Option<String>, ApiError> {
    value
        .map(|hex| {
            parse_lowercase_hex(&hex)
                .map(|_| hex)
                .map_err(|_| ApiError::unavailable())
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
        assert_eq!(
            json_u64(rows.into_iter().next().unwrap().networkmagic),
            Some(764824073)
        );
    }

    #[test]
    fn koios_collateral_accepts_json_encoded_assets() {
        let io: KoiosIo = serde_json::from_str(
            r#"{"asset_list":"[]","payment_addr":{"bech32":"addr_test1..."}}"#,
        )
        .unwrap();
        assert!(io.asset_list.is_empty());
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
