use anyhow::{Error, Result};
use cardano_tx_builder::PlutusData;

use crate::base::{Amount, Index, Lock, Timestamp};

#[derive(Debug, Clone)]
pub struct ChequeBody {
    index: Index,
    amount: Amount,
    timeout: Timestamp,
    lock: Lock,
}

impl ChequeBody {
    pub fn new(index: Index, amount: Amount, timeout: Timestamp, lock: Lock) -> Self {
        Self {
            index,
            amount,
            timeout,
            lock,
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ChequeBody {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            Index::try_from(a)?,
            Amount::try_from(b)?,
            Timestamp::try_from(c)?,
            Lock::try_from(d)?,
        ))
    }
}

impl<'a> From<ChequeBody> for PlutusData<'a> {
    fn from(value: ChequeBody) -> Self {
        Self::list(vec![
            PlutusData::from(value.index),
            PlutusData::from(value.amount),
            PlutusData::from(value.timeout),
            PlutusData::from(value.lock),
        ])
    }
}
