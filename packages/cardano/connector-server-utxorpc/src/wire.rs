use cardano_connector_utxorpc::{BloxbeanPayload, MappedUtxo};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct NetworkResponse {
    pub network: &'static str,
}

#[derive(Serialize)]
pub struct ProtocolParametersResponse {
    pub era: String,
    pub epoch: u64,
    pub slot: u64,
    pub payload: BloxbeanJson,
}

#[derive(Serialize)]
pub struct BloxbeanJson {
    pub min_fee_a: u64,
    pub min_fee_b: u64,
    pub max_tx_size: u64,
    pub key_deposit: String,
    pub pool_deposit: String,
    pub min_pool_cost: String,
    pub protocol_major_ver: u32,
    pub protocol_minor_ver: u32,
    pub coins_per_utxo_size: String,
    pub collateral_percent: u64,
    pub max_collateral_inputs: u64,
}

impl From<BloxbeanPayload> for BloxbeanJson {
    fn from(value: BloxbeanPayload) -> Self {
        Self {
            min_fee_a: value.min_fee_a,
            min_fee_b: value.min_fee_b,
            max_tx_size: value.max_tx_size,
            key_deposit: value.key_deposit.to_string(),
            pool_deposit: value.pool_deposit.to_string(),
            min_pool_cost: value.min_pool_cost.to_string(),
            protocol_major_ver: value.protocol_major_ver,
            protocol_minor_ver: value.protocol_minor_ver,
            coins_per_utxo_size: value.coins_per_utxo_size.to_string(),
            collateral_percent: value.collateral_percent,
            max_collateral_inputs: value.max_collateral_inputs,
        }
    }
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub lovelace: String,
}

#[derive(Serialize, Clone)]
pub struct AssetObject {
    pub unit: String,
    pub quantity: String,
}

#[derive(Serialize, Clone)]
pub struct TxOutput {
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed_by: Option<String>,
    pub value: Vec<AssetObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_inline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_script_hash: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct TxInput {
    pub transaction_id: String,
    pub output_index: u32,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed_by: Option<String>,
    pub value: Vec<AssetObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_inline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_script_hash: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Utxo {
    pub transaction_id: String,
    pub output_index: u32,
    pub address: String,
    pub value: Vec<AssetObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datum_inline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_script_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_script_version: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_script: Option<String>,
}

impl From<MappedUtxo> for Utxo {
    fn from(utxo: MappedUtxo) -> Self {
        Self {
            transaction_id: hex::encode(utxo.transaction_id),
            output_index: utxo.output_index,
            address: utxo.address,
            value: utxo
                .value
                .into_iter()
                .map(|asset| AssetObject {
                    unit: asset.unit,
                    quantity: asset.quantity.to_string(),
                })
                .collect(),
            datum_hash: utxo.datum_hash.map(hex::encode),
            datum_inline: utxo.datum_inline.map(hex::encode),
            reference_script_hash: utxo.reference_script_hash.map(hex::encode),
            reference_script_version: utxo.reference_script_version,
            reference_script: utxo.reference_script.map(hex::encode),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TransactionSummary {
    pub id: String,
    pub index: u64,
    pub depth: u64,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_before: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_after: Option<u64>,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitRequest {
    pub transaction: String,
}

#[derive(Serialize)]
pub struct SubmitResponse {
    pub transaction_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateOperationRequest {
    pub operation_id: String,
    pub expected_transaction_id: String,
    pub transaction: String,
}

#[derive(Serialize, Clone)]
pub struct OperationResponse {
    pub operation_id: String,
    pub expected_transaction_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,
    pub status: &'static str,
    pub depth: u64,
}
