use crate::Secret;
use cardano_tx_builder::PlutusData;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Lock(pub [u8; 32]);

impl Lock {
    pub fn from_secret(s: Secret) -> Self {
        Self(Sha256::digest(s.0).into())
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Lock {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array = <&'_ [u8; 32]>::try_from(data).map_err(|e| e.context("invalid lock"))?;
        Ok(Self(*array))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Lock {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<Lock> for PlutusData<'a> {
    fn from(value: Lock) -> Self {
        Self::bytes(value.0)
    }
}
