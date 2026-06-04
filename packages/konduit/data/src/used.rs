use serde::{Deserialize, Serialize};

use crate::{Unlocked, VerifyState};

#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Used {
    pub index: u64,
    pub amount: u64,
}

impl Used {
    pub fn new(index: u64, amount: u64) -> Self {
        Self { index, amount }
    }
}

impl<V: VerifyState> From<&Unlocked<V>> for Used {
    fn from(value: &Unlocked<V>) -> Self {
        Self::new(value.index(), value.amount())
    }
}

/// On-chain encoding: an indefinite-length array of [index, amount].
impl<C> minicbor::Encode<C> for Used {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(self.index, ctx)?;
        e.encode_with(self.amount, ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Used {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let index: u64 = d.decode_with(ctx)?;
        let amount: u64 = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Used array",
            ));
        }
        d.skip()?;
        Ok(Self::new(index, amount))
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<Vec<PlutusData<'a>>> for Used {
        type Error = anyhow::Error;

        fn try_from(list: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
            let [a, b] = <[PlutusData; 2]>::try_from(list)
                .map_err(|_| anyhow!("invalid 'Used': expected 2-element list"))?;
            Ok(Self::new(u64::try_from(&a)?, u64::try_from(&b)?))
        }
    }

    impl<'a> TryFrom<&PlutusData<'a>> for Used {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(Vec::try_from(data)?)
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Used {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Used> for PlutusData<'a> {
        fn from(used: Used) -> Self {
            Self::list(vec![
                PlutusData::from(used.index),
                PlutusData::from(used.amount),
            ])
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
        /// minicbor encodes and decodes Used back to the same value.
        #[test]
        fn cbor(val: Used) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Used = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Used) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Used) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Used = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Used> for PlutusData and TryFrom<PlutusData> for Used are mutual inverses.
        #[test]
        fn tryfrom(val: Used) {
            let pd = PlutusData::from(val.clone());
            let recovered = Used::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
