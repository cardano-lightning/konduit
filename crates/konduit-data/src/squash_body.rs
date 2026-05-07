use crate::{ChequeBody, Indexes, IndexesError, Tag};
use anyhow::anyhow;
use cardano_sdk::{PlutusData, cbor, cbor::ToCbor};
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SquashBodyError {
    #[error("Duplicate index")]
    DuplicateIndex,

    #[error("Exclude error {0}")]
    Exclude(IndexesError),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SquashBody {
    pub amount: u64,
    pub index: u64,
    pub exclude: Indexes,
}

impl Serialize for SquashBody {
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

impl<'de> Deserialize<'de> for SquashBody {
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

impl SquashBody {
    pub fn new(amount: u64, index: u64, exclude: Indexes) -> anyhow::Result<Self> {
        match SquashBody::verify_new(&index, &exclude) {
            true => Ok(SquashBody::new_no_verify(amount, index, exclude)),
            false => Err(anyhow!("Index must be greater than excludes")),
        }
    }

    pub fn verify_new(index: &u64, exclude: &Indexes) -> bool {
        match exclude.0.last() {
            Some(last) => last < index,
            None => true,
        }
    }

    pub fn new_no_verify(amount: u64, index: u64, exclude: Indexes) -> Self {
        Self {
            amount,
            index,
            exclude,
        }
    }

    pub fn tagged_bytes(&self, tag: &Tag) -> Vec<u8> {
        let mut data = PlutusData::from(self).to_cbor();
        let mut x = tag.as_ref().to_vec();
        x.append(&mut data);
        x
    }

    /// Only squash what has been verified.
    /// Fails if the cheque is unable to be squashed due
    /// to: Already squashed or; exceed max exclude length.
    pub fn squash(&mut self, cheque_body: ChequeBody) -> Result<(), SquashBodyError> {
        match self.exclude.remove(cheque_body.index) {
            Ok(_) => {
                self.amount += cheque_body.amount;
                Ok(())
            }
            Err(_) => match self.index < cheque_body.index {
                false => Err(SquashBodyError::DuplicateIndex),
                true => {
                    self.exclude
                        .extend(self.index + 1, cheque_body.index - 1)
                        .map_err(SquashBodyError::Exclude)?;
                    self.amount += cheque_body.amount;
                    self.index = cheque_body.index;
                    Ok(())
                }
            },
        }
    }

    pub fn is_index_squashed(&self, index: u64) -> bool {
        match self.index.cmp(&index) {
            cmp::Ordering::Less => false,
            cmp::Ordering::Equal => true,
            cmp::Ordering::Greater => !self.exclude.has(index),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for SquashBody {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let [a, b, c] = <[PlutusData; 3]>::try_from(&data)?;
        Self::new(
            <u64>::try_from(&a)?,
            <u64>::try_from(&b)?,
            Indexes::try_from(c)?,
        )
    }
}

impl<'a> From<&SquashBody> for PlutusData<'a> {
    fn from(value: &SquashBody) -> Self {
        Self::list(vec![
            PlutusData::from(value.amount),
            PlutusData::from(value.index),
            PlutusData::from(&value.exclude),
        ])
    }
}

impl<'a> From<SquashBody> for PlutusData<'a> {
    fn from(value: SquashBody) -> Self {
        Self::list(vec![
            PlutusData::from(value.amount),
            PlutusData::from(value.index),
            PlutusData::from(&value.exclude),
        ])
    }
}
