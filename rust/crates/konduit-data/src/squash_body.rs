use anyhow::{Error, Result};
use cardano_tx_builder::PlutusData;

use crate::{
    base::{Amount, Index},
    exclude::Exclude,
};

#[derive(Debug, Clone)]
pub struct SquashBody {
    amount: Amount,
    index: Index,
    exclude: Exclude,
}

impl SquashBody {
    pub fn new(amount: Amount, index: Index, exclude: Exclude) -> Self {
        Self {
            amount,
            index,
            exclude,
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for SquashBody {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c] = <[PlutusData; 3]>::try_from(&data)?;
        Ok(Self::new(
            Amount::try_from(a)?,
            Index::try_from(b)?,
            Exclude::try_from(&c)?,
        ))
    }
}

impl<'a> From<SquashBody> for PlutusData<'a> {
    fn from(value: SquashBody) -> Self {
        Self::list(vec![
            PlutusData::from(value.amount),
            PlutusData::from(value.index),
            PlutusData::from(value.exclude),
        ])
    }
}
