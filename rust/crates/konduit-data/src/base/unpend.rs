use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

use crate::{Secret, utils::try_into_array};

#[derive(Debug, Clone)]
pub enum Unpend {
    Continue,
    Expire,
    Unlock([u8; 32]),
}

impl Unpend {
    pub fn is_continue(&self) -> bool {
        matches!(self, Unpend::Continue)
    }
}

impl From<&Secret> for Unpend {
    fn from(value: &Secret) -> Self {
        Self::Unlock(value.0.clone())
    }
}

impl std::str::FromStr for Unpend {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if s.is_empty() {
            return Ok(Unpend::Continue);
        }
        Ok(Unpend::Unlock(try_into_array(
            &hex::decode(s).map_err(|e| anyhow!(e).context("invalid unpend"))?,
        )?))
    }
}

// We use either empty bytestring or a 32-bytestring to represent Unpend
impl<'a> TryFrom<&PlutusData<'a>> for Unpend {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let bytes: &[u8] = <&[u8]>::try_from(data).map_err(|e| e.context("invalid unpend"))?;
        match bytes.len() {
            0 => Ok(Unpend::Continue),
            32 => {
                let arr = <[u8; 32]>::try_from(bytes)?;
                Ok(Unpend::Unlock(arr))
            }
            _ => Ok(Unpend::Expire),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Unpend {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

impl<'a> From<Unpend> for PlutusData<'a> {
    fn from(value: Unpend) -> Self {
        match value {
            Unpend::Continue => PlutusData::bytes([]),
            Unpend::Expire => PlutusData::bytes([0]),
            Unpend::Unlock(arr) => PlutusData::bytes(arr),
        }
    }
}
