use crate::{Lock, Timestamp};
use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone)]
pub struct Pending {
    pub amount: u64,
    pub timeout: Timestamp,
    pub lock: Lock,
}

impl Pending {
    pub fn new(amount: u64, timeout: Timestamp, lock: Lock) -> Self {
        Self {
            amount,
            timeout,
            lock,
        }
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Pending {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b, c] =
            <[PlutusData; 3]>::try_from(list).map_err(|_| anyhow!("invalid 'Pending'"))?;
        Ok(Self::new(
            u64::try_from(&a)?,
            Timestamp::try_from(b)?,
            Lock::try_from(c)?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Pending {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = Vec::try_from(data)?;
        Self::try_from(list)
    }
}

impl<'a> From<Pending> for Vec<PlutusData<'a>> {
    fn from(pending: Pending) -> Self {
        vec![
            PlutusData::from(pending.amount),
            PlutusData::from(pending.timeout),
            PlutusData::from(pending.lock),
        ]
    }
}

impl<'a> From<Pending> for PlutusData<'a> {
    fn from(pending: Pending) -> Self {
        Self::list(Vec::from(pending))
    }
}
