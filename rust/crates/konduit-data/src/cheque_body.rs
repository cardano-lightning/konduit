use anyhow::{Error, Result};
use cardano_tx_builder::{cbor::ToCbor, PlutusData};

use crate::base::{Amount, Index, Lock, Tag, Timestamp};

#[derive(Debug, Clone)]
pub struct ChequeBody {
    pub index: Index,
    pub amount: Amount,
    pub timeout: Timestamp,
    pub lock: Lock,
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

    pub fn to_tagged_bytes(&self, tag : Tag) -> Vec<u8> {
        let mut data = PlutusData::from(self.clone()).to_cbor();
        let mut x = tag.0.clone();
        x.append(&mut data);
        x 
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
        Self::list([
            PlutusData::from(value.index),
            PlutusData::from(value.amount),
            PlutusData::from(value.timeout),
            PlutusData::from(value.lock),
        ])
    }
}
