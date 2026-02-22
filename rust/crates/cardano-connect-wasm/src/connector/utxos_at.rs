use crate::{
    asset_object::{AssetObject, from_asset_objects},
    helpers::try_into_array,
};
use anyhow::anyhow;
use cardano_tx_builder::{
    Address, Hash, Input, Output, PlutusScript, PlutusVersion, address::kind, cbor,
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
