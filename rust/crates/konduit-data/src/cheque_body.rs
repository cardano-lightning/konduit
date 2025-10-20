use anyhow::{Error, Result};
use cardano_tx_builder::{PlutusData, cbor::ToCbor};
use cryptoxide::hashing::sha256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChequeBody {
    pub index: u64,
    pub amount: u64,
    pub timeout: u64,
    pub lock: [u8; 32],
}

impl ChequeBody {
    pub fn new(index: u64, amount: u64, timeout: u64, lock: [u8; 32]) -> Self {
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

    pub fn is_secret(&self, secret: &[u8; 32]) -> bool {
        self.lock == sha256(secret)
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
            <&[u8; 32]>::try_from(&d)?.clone(),
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
