use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::base::{Amount, Signature};
use crate::squash_body::SquashBody;

#[derive(Debug, Clone)]
pub struct Squash {
    pub squash_body: SquashBody,
    pub signature: Signature,
}

impl Squash {
    pub fn new(squash_body: SquashBody, signature: Signature) -> Self {
        Self {
            squash_body,
            signature,
        }
    }

    pub fn amount(&self) -> Amount {
        self.squash_body.amount.clone()
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Squash {
    type Error = Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for Squash {
    type Error = Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> Result<Self> {
        let [a, b] = value;
        Ok(Self::new(SquashBody::try_from(a)?, Signature::try_from(b)?))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Squash {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<Squash> for [PlutusData<'a>; 2] {
    fn from(value: Squash) -> Self {
        [
            PlutusData::from(value.squash_body),
            PlutusData::from(value.signature),
        ]
    }
}

impl<'a> From<Squash> for PlutusData<'a> {
    fn from(value: Squash) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}
