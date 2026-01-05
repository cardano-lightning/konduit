use crate::{Duration, Lock, Secret, Tag};
use anyhow::{Error, Result};
use cardano_tx_builder::{
    PlutusData,
    cbor::{self, ToCbor},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChequeBody {
    pub index: u64,
    pub amount: u64,
    pub timeout: Duration,
    pub lock: Lock,
}

impl Serialize for ChequeBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = PlutusData::from(self.clone()).to_cbor();
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for ChequeBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = if deserializer.is_human_readable() {
            let str: String = serde::Deserialize::deserialize(deserializer)?;
            hex::decode(str).map_err(serde::de::Error::custom)?
        } else {
            serde::Deserialize::deserialize(deserializer)?
        };
        let plutus_data: PlutusData = cbor::decode(&bytes).map_err(serde::de::Error::custom)?;
        Self::try_from(plutus_data).map_err(serde::de::Error::custom)
    }
}

impl ChequeBody {
    pub fn new(index: u64, amount: u64, timeout: Duration, lock: Lock) -> Self {
        Self {
            index,
            amount,
            timeout,
            lock,
        }
    }

    pub fn tagged_bytes(&self, tag: &Tag) -> Vec<u8> {
        let mut data = PlutusData::from(self.clone()).to_cbor();
        let mut x = tag.0.clone();
        x.append(&mut data);
        x
    }

    pub fn is_secret(&self, secret: &Secret) -> bool {
        self.lock == Lock::from(secret.clone())
    }
}

impl<'a> TryFrom<PlutusData<'a>> for ChequeBody {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let [a, b, c, d] = <[PlutusData; 4]>::try_from(&data)?;
        Ok(Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            Duration::try_from(&c)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cheque_body_round_trip() {
        let original = ChequeBody {
            index: 42,
            amount: 5000000,
            timeout: Duration::from_secs(123002302),
            lock: Lock([1; 32]),
        };

        let ser = serde_json::to_vec(&original).expect("Failed to serialize ChequeBody");

        let de: ChequeBody =
            serde_json::from_slice(&ser).expect("Failed to deserialize ChequeBody");

        assert_eq!(original.index, de.index);
        assert_eq!(original.amount, de.amount);
        assert_eq!(original.timeout, de.timeout);
        assert_eq!(original.lock, de.lock);
    }
}
