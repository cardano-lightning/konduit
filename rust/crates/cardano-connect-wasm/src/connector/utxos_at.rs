use anyhow::anyhow;
use cardano_tx_builder::{
    Address, Credential, Hash, Input, Output, PlutusScript, PlutusVersion, Value, address::kind,
    cbor,
};

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    pub transaction_id: String,
    pub output_index: u64,
    pub address: String,
    pub value: Vec<AssetObject>,
    pub datum_hash: Option<String>,
    pub datum_inline: Option<String>,
    pub reference_script_version: Option<u8>,
    pub reference_script: Option<String>,
}

impl TryFrom<Response> for (Input, Output) {
    type Error = anyhow::Error;

    fn try_from(utxo: Response) -> anyhow::Result<Self> {
        let input = Input::new(
            try_into_array(hex::decode(&utxo.transaction_id)?)?.into(),
            utxo.output_index,
        );

        let address = <Address<kind::Shelley>>::try_from(utxo.address.as_str())?;

        let mut output = Output::new(address.into(), from_asset_objects(&utxo.value[..])?);

        if let Some(hash_str) = utxo.datum_hash.as_deref() {
            output = output.with_datum_hash(Hash::try_from(hash_str)?);
        }

        if let Some(data_str) = utxo.datum_inline {
            let plutus_data = cbor::decode(&hex::decode(data_str)?)?;
            output = output.with_datum(plutus_data);
        }

        if let Some(script_str) = utxo.reference_script {
            let plutus_script = hex::decode(script_str)
                .map_err(|e| anyhow!(e).context("malformed script reference"))?;

            let plutus_version = PlutusVersion::try_from(
                utxo.reference_script_version
                    .expect("missing version with script"),
            )?;

            output = output.with_plutus_script(PlutusScript::new(plutus_version, plutus_script));
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
