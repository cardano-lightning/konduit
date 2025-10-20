use crate::{Duration, Tag};
use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, VerificationKey};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Constants {
    pub tag: Tag,
    pub add_vkey: VerificationKey,
    pub sub_vkey: VerificationKey,
    pub close_period: Duration,
}

impl Constants {
    pub fn verify(&self, max_tag_length: usize, min_close_period: u64) -> bool {
        self.tag.0.len() <= max_tag_length
            && self.close_period.as_millis() >= min_close_period as u128
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Constants {
    type Error = anyhow::Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 4]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 4]> for Constants {
    type Error = anyhow::Error;

    fn try_from(value: [PlutusData<'a>; 4]) -> anyhow::Result<Self> {
        let [a, b, c, d] = value;
        Ok(Self {
            tag: Tag::try_from(&a)?,
            add_vkey: VerificationKey::from(*<&[u8; 32]>::try_from(&b)?),
            sub_vkey: VerificationKey::from(*<&[u8; 32]>::try_from(&c)?),
            close_period: Duration::try_from(&d)?,
        })
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Constants {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 4]>::try_from(data)?)
    }
}

impl<'a> From<Constants> for [PlutusData<'a>; 4] {
    fn from(value: Constants) -> Self {
        [
            PlutusData::from(value.tag),
            PlutusData::from(<[u8; 32]>::from(value.add_vkey)),
            PlutusData::from(<[u8; 32]>::from(value.sub_vkey)),
            PlutusData::from(value.close_period),
        ]
    }
}

impl<'a> From<Constants> for PlutusData<'a> {
    fn from(value: Constants) -> Self {
        Self::list(<[PlutusData; 4]>::from(value))
    }
}
