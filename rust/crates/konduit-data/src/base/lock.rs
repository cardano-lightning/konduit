use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, PlutusDataDecodeError};
use cryptoxide::hashing::sha256;
use serde::{Deserialize, Serialize};

use crate::{Secret, impl_hex_serde_for_wrapper, utils::try_into_array};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Lock(pub [u8; 32]);

impl std::str::FromStr for Lock {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Lock(try_into_array(
            &hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?,
        )?))
    }
}

impl_hex_serde_for_wrapper!(Lock, [u8; 32]);

impl From<Secret> for Lock {
    fn from(value: Secret) -> Self {
        Lock(sha256(&value.0))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Lock {
    type Error = PlutusDataDecodeError;

    fn try_from(data: &PlutusData<'a>) -> Result<Self, Self::Error> {
        Ok(Self(<&[u8; 32]>::try_from(data)?.to_owned()))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Lock {
    type Error = PlutusDataDecodeError;

    fn try_from(data: PlutusData<'a>) -> Result<Self, Self::Error> {
        Self::try_from(&data)
    }
}

impl<'a> From<Lock> for PlutusData<'a> {
    fn from(value: Lock) -> Self {
        Self::bytes(value.0)
    }
}
