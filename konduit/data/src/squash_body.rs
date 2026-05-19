use crate::{ChequeBody, Indexes, IndexesError};
use anyhow::anyhow;
use cardano_sdk::PlutusData;
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SquashBodyError {
    #[error("Duplicate index")]
    DuplicateIndex,

    #[error("Exclude error {0}")]
    Exclude(IndexesError),
}

#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SquashBody {
    #[cfg_attr(feature = "cddl", n(0))]
    pub amount: u64,
    #[cfg_attr(feature = "cddl", n(1))]
    pub index: u64,
    #[cfg_attr(feature = "cddl", n(2))]
    pub exclude: Indexes,
}

impl SquashBody {
    pub fn new(amount: u64, index: u64, exclude: Indexes) -> anyhow::Result<Self> {
        match SquashBody::verify_new(index, &exclude) {
            true => Ok(SquashBody::new_no_verify(amount, index, exclude)),
            false => Err(anyhow!("Index must be greater than excludes")),
        }
    }

    pub fn verify_new(index: u64, exclude: &Indexes) -> bool {
        match exclude.last() {
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

    /// Only squash what has been verified.
    /// Fails if the cheque is unable to be squashed due
    /// to: Already squashed or; exceed max exclude length.
    pub fn squash(&mut self, cheque_body: ChequeBody) -> Result<(), SquashBodyError> {
        match self.exclude.remove(cheque_body.index()) {
            Ok(_) => {
                self.amount += cheque_body.amount();
                Ok(())
            }
            Err(_) => match self.index < cheque_body.index() {
                false => Err(SquashBodyError::DuplicateIndex),
                true => {
                    self.exclude
                        .extend(self.index + 1, cheque_body.index() - 1)
                        .map_err(SquashBodyError::Exclude)?;
                    self.amount += cheque_body.amount();
                    self.index = cheque_body.index();
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

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for SquashBody {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        proptest::collection::btree_set(0u64..u64::MAX / 2, 0..=crate::MAX_EXCLUDE_LENGTH)
            .prop_flat_map(|set| {
                let exclude: Vec<u64> = set.into_iter().collect();
                let lo = exclude.last().map(|x| x + 1).unwrap_or(0);
                let hi = lo.saturating_add(1_000_000);
                (Just(exclude), lo..=hi, any::<u64>())
            })
            .prop_map(|(exclude, index, amount)| {
                SquashBody::new_no_verify(amount, index, Indexes(exclude))
            })
            .boxed()
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
