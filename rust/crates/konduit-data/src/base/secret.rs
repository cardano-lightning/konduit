use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, PlutusDataDecodeError};

use serde::{Deserialize, Serialize};

use crate::{impl_hex_serde_for_wrapper, utils::try_into_array};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Secret(pub [u8; 32]);

impl std::str::FromStr for Secret {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Secret(try_into_array(
            &hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?,
        )?))
    }
}

impl_hex_serde_for_wrapper!(Secret, [u8; 32]);

impl<'a> TryFrom<&PlutusData<'a>> for Secret {
    type Error = PlutusDataDecodeError;

    fn try_from(data: &PlutusData<'a>) -> Result<Self, Self::Error> {
        Ok(Self(<&[u8; 32]>::try_from(data)?.to_owned()))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Secret {
    type Error = PlutusDataDecodeError;

    fn try_from(data: PlutusData<'a>) -> Result<Self, Self::Error> {
        Self::try_from(&data)
    }
}

impl<'a> From<Secret> for PlutusData<'a> {
    fn from(value: Secret) -> Self {
        Self::bytes(value.0)
    }
}
