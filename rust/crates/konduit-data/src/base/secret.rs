use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Secret(pub [u8; 32]);

impl<'a> TryFrom<&PlutusData<'a>> for Secret {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array = <&'_ [u8; 32]>::try_from(data).map_err(|e| e.context("invalid secret"))?;
        Ok(Self(*array))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Secret {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<Secret> for PlutusData<'a> {
    fn from(value: Secret) -> Self {
        Self::bytes(value.0)
    }
}
