use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Signature(pub [u8; 64]);

impl<'a> TryFrom<&PlutusData<'a>> for Signature {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array = <&'_ [u8; 64]>::try_from(data).map_err(|e| e.context("invalid signature"))?;
        Ok(Self(*array))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Signature {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<Signature> for PlutusData<'a> {
    fn from(value: Signature) -> Self {
        Self::bytes(value.0)
    }
}
