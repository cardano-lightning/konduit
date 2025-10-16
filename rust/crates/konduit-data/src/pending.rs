use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::base::{Amount, Lock, Timestamp};

#[derive(Debug, Clone)]
pub struct Pending {
    amount: Amount,
    timeout: Timestamp,
    lock: Lock,
}

impl Pending {
    pub fn new(amount: Amount, timeout: Timestamp, lock: Lock) -> Self {
        Self {
            amount,
            timeout,
            lock,
        }
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 3]> for Pending {
    type Error = Error;

    fn try_from(value: [PlutusData<'a>; 3]) -> Result<Self> {
        let [a, b, c] = value;
        Ok(Self::new(
            Amount::try_from(a)?,
            Timestamp::try_from(b)?,
            Lock::try_from(c)?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Pending {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        Self::try_from(<[PlutusData; 3]>::try_from(data)?)
    }
}

impl<'a> From<Pending> for PlutusData<'a> {
    fn from(value: Pending) -> Self {
        Self::list(vec![
            PlutusData::from(value.amount),
            PlutusData::from(value.timeout),
            PlutusData::from(value.lock),
        ])
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Pending {
    type Error = Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> Result<Self> {
        Self::try_from(<[PlutusData; 3]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}
