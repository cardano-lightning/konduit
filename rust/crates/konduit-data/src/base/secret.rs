use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

use crate::utils::try_into_array;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Secret(pub [u8; 32]);

impl std::str::FromStr for Secret {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Secret(try_into_array(
            &hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?,
        )?))
    }
}

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for Secret {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(
            <[u8; 32]>::try_from(value).map_err(|_| anyhow::anyhow!("Wrong length"))?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Secret {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let v = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid tag"))?;
        Ok(Self(try_into_array(v)?))
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
