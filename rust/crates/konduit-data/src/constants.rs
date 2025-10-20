use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::{PlutusData, VerificationKey};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Constants {
    pub tag: Vec<u8>,
    pub add_vkey: VerificationKey,
    pub sub_vkey: VerificationKey,
    pub close_period: u64,
}

impl Constants {
    pub fn new(
        tag: Vec<u8>,
        add_vkey: VerificationKey,
        sub_vkey: VerificationKey,
        close_period: u64,
    ) -> Self {
        Self {
            tag,
            add_vkey,
            sub_vkey,
            close_period,
        }
    }

    pub fn verify(&self, max_tag_length: usize, min_close_period: u64) -> bool {
        self.tag.len() <= max_tag_length && self.close_period >= min_close_period
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
            <&[u8]>::try_from(&a)?.to_vec(),
            VerificationKey::from(<&[u8; 32]>::try_from(&b)?.clone()),
            VerificationKey::from(<&[u8; 32]>::try_from(&c)?.clone()),
            <u64>::try_from(&d)?,
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
