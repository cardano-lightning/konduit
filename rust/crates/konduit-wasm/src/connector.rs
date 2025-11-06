use anyhow::anyhow;
use cardano_connect::{CardanoConnect, Network};
use cardano_tx_builder::{
    Address, Credential, Hash, Input, Output, ProtocolParameters, Transaction, Value,
    address::kind, transaction::state,
};
use gloo_net::http::Request;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CardanoConnector {
    base_url: String,
    network: Network,
}

#[wasm_bindgen]
impl CardanoConnector {
    #[wasm_bindgen]
    pub async fn new(base_url: &str) -> crate::Result<Self> {
        let base_url = base_url.strip_suffix("/").unwrap_or(base_url).to_string();

        let response = Request::get(&format!("{}/network", &base_url))
            .send()
            .await
            .map_err(|e| anyhow!(e))?;

        let json = response
            .json::<NetworkObject>()
            .await
            .map_err(|e| anyhow!(e))?;

        let network = match json.network.as_str() {
            "mainnet" => Network::Mainnet,
            "preprod" => Network::Preprod,
            "preview" => Network::Preview,
            _ => panic!("unexpected network returned by the connector"),
        };

        Ok(Self { base_url, network })
    }
}

impl CardanoConnect for CardanoConnector {
    fn network(&self) -> Network {
        self.network
    }

    async fn health(&self) -> anyhow::Result<String> {
        let response = Request::get(&format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| {
                log::error!("GET /health FAIL: {e:?}");
                anyhow!(e)
            })?;

        Ok(response
            .json::<HealthObject>()
            .await
            .expect("invalid json object")
            .status)
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        Ok(match self.network {
            Network::Mainnet => ProtocolParameters::mainnet(),
            Network::Preprod => ProtocolParameters::preprod(),
            Network::Preview => ProtocolParameters::preview(),
            Network::Other(..) => panic!("unexpected 'other' network"),
        })
    }

    /// If delegation is None then it _should_ be ignored:
    /// Any address with matching payment credential should be returned.
    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let mut addr = Address::new(self.network().into(), payment.clone());
        if let Some(delegation) = delegation {
            addr = addr.with_delegation(delegation.clone());
        }

        let response = Request::get(&format!("{}/utxos_at/{addr}", self.base_url))
            .send()
            .await
            .map_err(|e| {
                log::error!("GET /utxos_at/{addr} FAIL: {e:?}");
                anyhow!(e)
            })?;

        Ok(response
            .json::<Vec<UtxoObject>>()
            .await
            .expect("invalid UtxoObject")
            .into_iter()
            .map(|obj| <(Input, Output)>::try_from(obj).expect("failed to convert UtxoObject"))
            .collect())
    }

    async fn submit(
        &self,
        _transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<String> {
        todo!()
    }
}

#[derive(Debug, serde::Deserialize)]
struct HealthObject {
    status: String,
}

#[derive(Debug, serde::Deserialize)]
struct NetworkObject {
    network: String,
}

#[derive(Debug, serde::Deserialize)]
struct AssetObject {
    unit: String,
    quantity: String,
}

impl AssetObject {
    const UNIT_LOVELACE: &str = "lovelace";
}

fn from_asset_objects(assets: &[AssetObject]) -> anyhow::Result<Value<u64>> {
    fn from_asset_unit(unit: &str) -> anyhow::Result<(Hash<28>, Vec<u8>)> {
        let script_hash: [u8; Credential::DIGEST_SIZE] =
            try_into_array(hex::decode(&unit[0..2 * Credential::DIGEST_SIZE])?)?;

        let asset_name: Vec<u8> = hex::decode(&unit[2 * Credential::DIGEST_SIZE..])?;

        Ok((Hash::from(script_hash), asset_name))
    }

    let mut lovelace = None;
    let mut value = Vec::new();

    for asset in assets {
        let amount: u64 = asset.quantity.parse()?;
        if asset.unit == AssetObject::UNIT_LOVELACE {
            lovelace = Some(amount);
        } else {
            let (script_hash, asset_name) = from_asset_unit(&asset.unit)?;
            value.push((script_hash, [(asset_name, amount)]));
        }
    }

    Ok(Value::new(lovelace.unwrap_or_default()).with_assets(value))
}

#[derive(Debug, serde::Deserialize)]
struct UtxoObject {
    address: String,
    tx_hash: String,
    tx_index: u64,
    amount: Vec<AssetObject>,
    data_hash: Option<String>,
    inline_datum: Option<String>,
    reference_script_hash: Option<String>,
}

impl TryFrom<UtxoObject> for (Input, Output) {
    type Error = anyhow::Error;

    fn try_from(utxo: UtxoObject) -> anyhow::Result<Self> {
        let input = Input::new(
            try_into_array(hex::decode(&utxo.tx_hash)?)?.into(),
            utxo.tx_index,
        );

        let address = <Address<kind::Shelley>>::try_from(utxo.address.as_str())?;

        let output = Output::new(address.into(), from_asset_objects(&utxo.amount[..])?);

        if utxo.inline_datum.is_some() {
            unimplemented!("non-null inline_datum in UTxO: unimplemented")
        }

        if utxo.data_hash.is_some() {
            unimplemented!("non-null datum hash in UTxO: unimplemented")
        }

        if utxo.reference_script_hash.is_some() {
            unimplemented!("non-null script hash in UTxO: unimplemented")
        };

        Ok((input, output))
    }
}

fn try_into_array<T, const N: usize>(v: Vec<T>) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}
