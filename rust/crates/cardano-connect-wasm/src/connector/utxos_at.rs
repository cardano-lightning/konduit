use anyhow::anyhow;
use cardano_tx_builder::{Address, Credential, Hash, Input, Output, Value, address::kind};

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    pub address: String,
    pub tx_hash: String,
    pub tx_index: u64,
    pub amount: Vec<AssetObject>,
    pub data_hash: Option<String>,
    pub inline_datum: Option<String>,
    pub reference_script_hash: Option<String>,
}

impl TryFrom<Response> for (Input, Output) {
    type Error = anyhow::Error;

    fn try_from(utxo: Response) -> anyhow::Result<Self> {
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

#[derive(Debug, serde::Deserialize)]
pub struct AssetObject {
    pub unit: String,
    pub quantity: String,
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

fn try_into_array<T, const N: usize>(v: Vec<T>) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}
