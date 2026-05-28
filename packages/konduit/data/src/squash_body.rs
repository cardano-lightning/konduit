use serde::{Deserialize, Serialize};
use std::cmp;

use crate::{ChequeBody, Indexes, IndexesError};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SquashBodyError {
    #[error("Duplicate index")]
    DuplicateIndex,

    #[error("Exclude error {0}")]
    Exclude(IndexesError),

    #[error("Index must be greater than all excluded indexes")]
    IndexNotGreaterThanExcludes,
}

/// On-chain encoding: an indefinite-length array of [amount, index, exclude].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SquashBody {
    amount: u64,
    index: u64,
    exclude: Indexes,
}

impl SquashBody {
    pub fn index(&self) -> u64 {
        self.index
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn exclude(&self) -> &Indexes {
        &self.exclude
    }

    pub fn new(amount: u64, index: u64, exclude: Indexes) -> Result<Self, SquashBodyError> {
        match SquashBody::verify_new(index, &exclude) {
            true => Ok(SquashBody::new_no_verify(amount, index, exclude)),
            false => Err(SquashBodyError::IndexNotGreaterThanExcludes),
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

impl<C> minicbor::Encode<C> for SquashBody {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(self.amount, ctx)?;
        e.encode_with(self.index, ctx)?;
        e.encode_with(&self.exclude, ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for SquashBody {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let amount: u64 = d.decode_with(ctx)?;
        let index: u64 = d.decode_with(ctx)?;
        let exclude: Indexes = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of SquashBody array",
            ));
        }
        d.skip()?;
        Self::new(amount, index, exclude)
            .map_err(|e| minicbor::decode::Error::message(e.to_string()))
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

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<PlutusData<'a>> for SquashBody {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let [a, b, c] = <[PlutusData; 3]>::try_from(&data)
                .map_err(|_| anyhow!("invalid 'SquashBody': expected 3-element list"))?;
            Self::new(
                u64::try_from(&a)?,
                u64::try_from(&b)?,
                Indexes::try_from(c)?,
            )
            .map_err(Into::into)
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
            PlutusData::from(&value)
        }
    }
}

#[cfg(feature = "proptest")]
#[allow(unused_imports)]
mod roundtrip {
    use super::*;
    use cardano_sdk::{PlutusData, cbor::ToCbor};
    use proptest::prelude::*;

    proptest! {
        /// minicbor encodes and decodes SquashBody back to the same value.
        #[test]
        fn cbor(val: SquashBody) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: SquashBody = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: SquashBody) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(&val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: SquashBody) {
            let pd_bytes = PlutusData::from(&val).to_cbor();
            let recovered: SquashBody = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<&SquashBody> for PlutusData and TryFrom<PlutusData> for SquashBody are mutual inverses.
        #[test]
        fn tryfrom(val: SquashBody) {
            let pd = PlutusData::from(&val);
            let recovered = SquashBody::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
