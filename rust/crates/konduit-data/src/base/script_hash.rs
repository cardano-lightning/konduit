use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
pub struct ScriptHash(pub [u8; 28]);

impl<'a> TryFrom<&PlutusData<'a>> for ScriptHash {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array = <&'_ [u8; 28]>::try_from(data).map_err(|e| e.context("invalid script hash"))?;
        Ok(Self(*array))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ScriptHash {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<ScriptHash> for PlutusData<'a> {
    fn from(value: ScriptHash) -> Self {
        Self::bytes(value.0)
    }
}
