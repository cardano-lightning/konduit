use cardano_tx_builder::PlutusData;

use crate::crypto::blake2b_224;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct VerificationKey(pub [u8; 32]);

impl<'a> TryFrom<&PlutusData<'a>> for VerificationKey {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array =
            <&'_ [u8; 32]>::try_from(data).map_err(|e| e.context("invalid verification key"))?;
        Ok(Self(*array))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for VerificationKey {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<VerificationKey> for PlutusData<'a> {
    fn from(value: VerificationKey) -> Self {
        Self::bytes(value.0)
    }
}

impl VerificationKey {
    pub fn hash(&self) -> [u8; 28] {
        blake2b_224(&self.0)
    }
}
