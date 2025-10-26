use anyhow::{Error, Result};
use cardano_tx_builder::{PlutusData, cbor::ToCbor};

use crate::{Lock, Secret};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChequeBody {
    pub index: u64,
    pub amount: u64,
    pub timeout: u64,
    pub lock: Lock,
}

impl ChequeBody {
    pub fn new(index: u64, amount: u64, timeout: u64, lock: Lock) -> Self {
        Self {
            index,
            amount,
            timeout,
            lock,
        }
    }

    pub fn tagged_bytes(&self, tag: Vec<u8>) -> Vec<u8> {
        let mut data = PlutusData::from(self.clone()).to_cbor();
        let mut x = tag.clone();
        x.append(&mut data);
        x
    }

    pub fn is_secret(&self, secret: Secret) -> bool {
        self.lock == Lock::from(secret)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ChequeBody {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            <u64>::try_from(&c)?,
            Lock::try_from(&d)?,
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
