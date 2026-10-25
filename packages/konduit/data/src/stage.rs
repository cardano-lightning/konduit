#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Duration, Pending, Used};

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Stage {
    Opened(u64, Vec<Used>),
    Closed(u64, Vec<Used>, Duration),
    Responded(u64, Vec<Pending>),
}

impl Stage {
    pub fn is_opened(&self) -> bool {
        matches!(self, Stage::Opened(_, _))
    }

    pub fn label(&self) -> &str {
        match self {
            Stage::Opened(_, _) => "Opened",
            Stage::Closed(_, _, _) => "Closed",
            Stage::Responded(_, _) => "Responded",
        }
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding:
//   Opened    → CBOR tag 121 (constr 0) + indef-array [subbed, list(useds)]
//   Closed    → CBOR tag 122 (constr 1) + indef-array [subbed, list(useds), duration]
//   Responded → CBOR tag 123 (constr 2) + indef-array [pends_amount, list(pendings)]
//
// The inner Vec<Used>/Vec<Pending> use plutus_list encoding
// (empty → definite array(0), non-empty → indefinite array).
// =========================================================================
impl<C> minicbor::Encode<C> for Stage
where
    Used: minicbor::Encode<C>,
    Pending: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Stage::Opened(subbed, useds) => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.begin_array()?;
                e.encode_with(subbed, ctx)?;
                crate::cbor_with::plutus_list::encode(useds, e, ctx)?;
                e.end()?;
            }
            Stage::Closed(subbed, useds, duration) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?;
                e.encode_with(subbed, ctx)?;
                crate::cbor_with::plutus_list::encode(useds, e, ctx)?;
                e.encode_with(duration, ctx)?;
                e.end()?;
            }
            Stage::Responded(pends_amount, pendings) => {
                e.tag(minicbor::data::Tag::new(123))?;
                e.begin_array()?;
                e.encode_with(pends_amount, ctx)?;
                crate::cbor_with::plutus_list::encode(pendings, e, ctx)?;
                e.end()?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Stage
where
    Used: minicbor::Decode<'b, C>,
    Pending: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        let variant: u64 = match cbor_tag.as_u64() {
            121 => 0,
            122 => 1,
            123 => 2,
            n => {
                return Err(minicbor::decode::Error::message(format!(
                    "unknown Stage CBOR tag {n}: expected 121, 122, or 123"
                )));
            }
        };
        d.array()?;
        match variant {
            0 => {
                let subbed: u64 = d.decode_with(ctx)?;
                let useds: Vec<Used> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Stage::Opened array",
                    ));
                }
                d.skip()?;
                Ok(Stage::Opened(subbed, useds))
            }
            1 => {
                let subbed: u64 = d.decode_with(ctx)?;
                let useds: Vec<Used> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                let duration: Duration = d.decode_with(ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Stage::Closed array",
                    ));
                }
                d.skip()?;
                Ok(Stage::Closed(subbed, useds, duration))
            }
            2 => {
                let pends_amount: u64 = d.decode_with(ctx)?;
                let pendings: Vec<Pending> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Stage::Responded array",
                    ));
                }
                d.skip()?;
                Ok(Stage::Responded(pends_amount, pendings))
            }
            _ => unreachable!(),
        }
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Stage {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            (
                any::<u64>(),
                proptest::collection::vec(any::<Used>(), 0..=crate::MAX_UNSQUASHED)
            )
                .prop_map(|(subbed, useds)| Stage::Opened(subbed, useds)),
            (
                any::<u64>(),
                proptest::collection::vec(any::<Used>(), 0..=crate::MAX_UNSQUASHED),
                any::<Duration>()
            )
                .prop_map(|(subbed, useds, duration)| Stage::Closed(subbed, useds, duration)),
            (
                any::<u64>(),
                proptest::collection::vec(any::<Pending>(), 0..=crate::MAX_UNSQUASHED)
            )
                .prop_map(|(pends_amount, pendings)| Stage::Responded(pends_amount, pendings)),
        ]
        .boxed()
    }
}

// =========================================================================
// PlutusData Conversions (proptest-gated)
//
// Kept so that proptest roundtrip tests can compare minicbor output against
// the canonical PlutusData CBOR encoding byte-for-byte.
// =========================================================================
#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::{PlutusData, constr};

    impl<'a> TryFrom<PlutusData<'a>> for Stage {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;

            return match variant {
                0 => try_opened(fields).map_err(|e| e.context("invalid 'Opened' variant")),
                1 => try_closed(fields).map_err(|e| e.context("invalid 'Closed' variant")),
                2 => try_responded(fields).map_err(|e| e.context("invalid 'Responded' variant")),
                _ => Err(anyhow!("unknown variant: {variant}")),
            };

            fn try_opened(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
                let [subbed, b] = <[PlutusData; 2]>::try_from(fields)
                    .map_err(|vec| anyhow!("expected 2 fields, found {}", vec.len()))?;
                let useds: Vec<Used> = b
                    .as_list()
                    .ok_or(anyhow!("Expected list"))?
                    .map(|x| Used::try_from(&x))
                    .collect::<anyhow::Result<Vec<Used>>>()?;
                Ok(Stage::Opened(u64::try_from(&subbed)?, useds))
            }

            fn try_closed(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
                let [subbed, b, c] = <[PlutusData; 3]>::try_from(fields)
                    .map_err(|vec| anyhow!("expected 3 fields, found {}", vec.len()))?;
                let useds: Vec<Used> = b
                    .as_list()
                    .ok_or(anyhow!("Expected list"))?
                    .map(|x| Used::try_from(&x))
                    .collect::<anyhow::Result<Vec<Used>>>()?;
                Ok(Stage::Closed(
                    u64::try_from(&subbed)?,
                    useds,
                    Duration::try_from(&c)?,
                ))
            }

            fn try_responded(fields: Vec<PlutusData<'_>>) -> anyhow::Result<Stage> {
                let [pends_amount, b] = <[PlutusData; 2]>::try_from(fields)
                    .map_err(|vec| anyhow!("expected 2 fields, found {}", vec.len()))?;
                let pendings: Vec<Pending> = b
                    .as_list()
                    .ok_or(anyhow!("Expected list"))?
                    .map(|x| Pending::try_from(&x))
                    .collect::<anyhow::Result<Vec<Pending>>>()?;
                Ok(Stage::Responded(u64::try_from(&pends_amount)?, pendings))
            }
        }
    }

    impl<'a> From<Stage> for PlutusData<'a> {
        fn from(value: Stage) -> Self {
            match value {
                Stage::Opened(subbed, b) => constr!(0, subbed, PlutusData::list(b)),
                Stage::Closed(subbed, b, c) => {
                    constr!(1, subbed, PlutusData::list(b), c)
                }
                Stage::Responded(pends_amount, b) => {
                    constr!(2, pends_amount, PlutusData::list(b))
                }
            }
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
        /// minicbor encodes and decodes Stage back to the same value.
        #[test]
        fn cbor(val: Stage) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Stage = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Stage) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Stage) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Stage = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Stage> for PlutusData and TryFrom<PlutusData> for Stage are mutual inverses.
        #[test]
        fn tryfrom(val: Stage) {
            let pd = PlutusData::from(val.clone());
            let recovered = Stage::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
