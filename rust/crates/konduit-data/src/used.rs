use anyhow::{Error, Result, anyhow};
use cardano_sdk::PlutusData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Used {
    pub index: u64,
    pub amount: u64,
}

impl Used {
    pub fn new(index: u64, amount: u64) -> Self {
        Self { index, amount }
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Used {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b] = <[PlutusData; 2]>::try_from(list).map_err(|_| anyhow!("invalid 'Used'"))?;
        Ok(Self::new(u64::try_from(&a)?, u64::try_from(&b)?))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Used {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = Vec::try_from(data)?;
        Self::try_from(list)
    }
}

impl<'a> From<Used> for Vec<PlutusData<'a>> {
    fn from(used: Used) -> Self {
        vec![PlutusData::from(used.index), PlutusData::from(used.amount)]
    }
}

impl<'a> From<Used> for PlutusData<'a> {
    fn from(used: Used) -> Self {
        Self::list(Vec::from(used))
    }
}
