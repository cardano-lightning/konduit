use crate::{Duration, Tag, VerifyingKey};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Constants {
    pub tag: Tag,
    pub add_vkey: VerifyingKey,
    pub sub_vkey: VerifyingKey,
    pub close_period: Duration,
}

impl Constants {
    pub fn verify(&self, max_tag_length: usize, min_close_period: u64) -> bool {
        self.tag.len() <= max_tag_length
            && self.close_period.as_millis() >= min_close_period as u128
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding: CBOR tag 121 (Plutus constr 0) followed by an indefinite-length
// array of [tag_bytes, add_vkey_bytes, sub_vkey_bytes, close_period_millis].
// =========================================================================
impl<C> minicbor::Encode<C> for Constants {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(minicbor::data::Tag::new(121))?;
        e.begin_array()?;
        e.encode_with(&self.tag, ctx)?;
        e.encode_with(self.add_vkey, ctx)?;
        e.encode_with(self.sub_vkey, ctx)?;
        e.encode_with(self.close_period, ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Constants {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let tag = d.tag()?;
        if tag.as_u64() != 121 {
            return Err(minicbor::decode::Error::message(
                "expected CBOR tag 121 for Constants",
            ));
        }
        d.array()?;
        let tag_val: Tag = d.decode_with(ctx)?;
        let add_vkey: VerifyingKey = d.decode_with(ctx)?;
        let sub_vkey: VerifyingKey = d.decode_with(ctx)?;
        let close_period: Duration = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Constants array",
            ));
        }
        d.skip()?;
        Ok(Self {
            tag: tag_val,
            add_vkey,
            sub_vkey,
            close_period,
        })
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Constants {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<Tag>(),
            any::<[u8; 32]>(),
            any::<[u8; 32]>(),
            any::<Duration>(),
        )
            .prop_map(|(tag, add_bytes, sub_bytes, close_period)| Constants {
                tag,
                add_vkey: VerifyingKey::from_bytes(add_bytes),
                sub_vkey: VerifyingKey::from_bytes(sub_bytes),
                close_period,
            })
            .boxed()
    }
}

// =========================================================================
// PlutusData Conversions (cardano_sdk-gated)
// =========================================================================
#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::{PlutusData, constr};

    impl<'a> TryFrom<&PlutusData<'a>> for Constants {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
            if tag != 0 {
                return Err(anyhow!("Bad constructor tag: expected 0, got {tag}"));
            }
            let [a, b, c, d] = <[PlutusData; 4]>::try_from(fields.collect::<Vec<_>>())
                .map_err(|_| anyhow!("invalid 'Constants': expected 4 fields"))?;
            Ok(Self {
                tag: Tag::try_from(&a)?,
                add_vkey: VerifyingKey::from_bytes(*<&[u8; 32]>::try_from(&b)?),
                sub_vkey: VerifyingKey::from_bytes(*<&[u8; 32]>::try_from(&c)?),
                close_period: Duration::try_from(&d)?,
            })
        }
    }

    impl<'a> From<Constants> for PlutusData<'a> {
        fn from(value: Constants) -> Self {
            constr!(
                0,
                PlutusData::from(value.tag),
                PlutusData::bytes(value.add_vkey.to_bytes()),
                PlutusData::bytes(value.sub_vkey.to_bytes()),
                PlutusData::from(value.close_period),
            )
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
        /// minicbor encodes and decodes Constants back to the same value.
        #[test]
        fn cbor(val: Constants) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Constants = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Constants) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Constants) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Constants = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Constants> for PlutusData and TryFrom<&PlutusData> for Constants are mutual inverses.
        #[test]
        fn tryfrom(val: Constants) {
            let pd = PlutusData::from(val.clone());
            let recovered = Constants::try_from(&pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
