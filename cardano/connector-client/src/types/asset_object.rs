use crate::helpers::try_into_array;
use cardano_sdk::{Credential, Hash, Value};

#[derive(Debug, serde::Deserialize)]
pub struct AssetObject {
    pub unit: String,
    pub quantity: String,
}

impl AssetObject {
    const UNIT_LOVELACE: &str = "lovelace";
}

pub fn from_asset_objects(assets: &[AssetObject]) -> anyhow::Result<Value<u64>> {
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
