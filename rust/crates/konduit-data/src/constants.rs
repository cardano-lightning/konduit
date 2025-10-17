use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::base::{Tag, TimeDelta, VerificationKey};

#[derive(Debug, Clone)]
pub struct Constants {
    pub tag: Tag,
    pub add_vkey: VerificationKey,
    pub sub_vkey: VerificationKey,
    pub close_period: TimeDelta,
}

impl Constants {
    pub fn new(
        tag: Tag,
        add_vkey: VerificationKey,
        sub_vkey: VerificationKey,
        close_period: TimeDelta,
    ) -> Self {
        Self {
            tag,
            add_vkey,
            sub_vkey,
            close_period,
        }
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Constants {
    type Error = Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> Result<Self> {
        Self::try_from(<[PlutusData; 4]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 4]> for Constants {
    type Error = Error;

    fn try_from(value: [PlutusData<'a>; 4]) -> Result<Self> {
        let [a, b, c, d] = value;
        Ok(Self::new(
            Tag::try_from(a)?,
            VerificationKey::try_from(b)?,
            VerificationKey::try_from(c)?,
            TimeDelta::try_from(d)?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Constants {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        Self::try_from(<[PlutusData; 4]>::try_from(data)?)
    }
}

impl<'a> From<Constants> for [PlutusData<'a>; 4] {
    fn from(value: Constants) -> Self {
        [
            PlutusData::from(value.tag),
            PlutusData::from(value.add_vkey),
            PlutusData::from(value.sub_vkey),
            PlutusData::from(value.close_period),
        ]
    }
}

impl<'a> From<Constants> for PlutusData<'a> {
    fn from(value: Constants) -> Self {
        Self::list(<[PlutusData; 4]>::from(value).to_vec())
    }
}
