use crate::{Constants, Stage};
use cardano_sdk::Hash;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Datum {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub own_hash: Hash<28>,
    pub constants: Constants,
    pub stage: Stage,
}

impl Datum {
    pub fn new(own_hash: Hash<28>, constants: Constants, stage: Stage) -> Self {
        Self {
            own_hash,
            constants,
            stage,
        }
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding: indefinite-length array of [own_hash_bytes, constants, stage].
// Hash<28> encodes as raw bytes via its own minicbor impl.
// =========================================================================
impl<C> minicbor::Encode<C> for Datum
where
    Constants: minicbor::Encode<C>,
    Stage: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(&self.own_hash, ctx)?;
        e.encode_with(&self.constants, ctx)?;
        e.encode_with(&self.stage, ctx)?;
        e.end()?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Datum
where
    Constants: minicbor::Decode<'b, C>,
    Stage: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let own_hash: Hash<28> = d.decode_with(ctx)?;
        let constants: Constants = d.decode_with(ctx)?;
        let stage: Stage = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Datum array",
            ));
        }
        d.skip()?;
        Ok(Self {
            own_hash,
            constants,
            stage,
        })
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Datum {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<[u8; 28]>(), any::<Constants>(), any::<Stage>())
            .prop_map(|(hash_bytes, constants, stage)| Datum {
                own_hash: Hash::from(hash_bytes),
                constants,
                stage,
            })
            .boxed()
    }
}

// =========================================================================
// PlutusData Conversions (proptest-gated)
//
// Kept so that proptest roundtrip tests can compare minicbor output against
// the canonical PlutusData CBOR encoding byte-for-byte.
// =========================================================================
#[cfg(feature = "proptest")]
mod via_plutus_data {
    use super::*;
    use anyhow::anyhow;
    use cardano_sdk::{PlutusData, cbor::ToCbor};

    impl<'a> TryFrom<&PlutusData<'a>> for Datum {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let items: Vec<PlutusData<'_>> = data
                .as_list()
                .ok_or(anyhow!("expected list for Datum"))?
                .collect();
            let [a, b, c]: [PlutusData<'_>; 3] = items
                .try_into()
                .map_err(|_| anyhow!("expected 3 fields in Datum"))?;
            Ok(Self {
                own_hash: Hash::from(*<&[u8; 28]>::try_from(&a)?),
                constants: Constants::try_from(&b)?,
                stage: Stage::try_from(c)?,
            })
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Datum {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Datum> for PlutusData<'a> {
        fn from(value: Datum) -> Self {
            Self::list(vec![
                PlutusData::from(&<[u8; 28]>::from(value.own_hash)),
                PlutusData::from(value.constants),
                PlutusData::from(value.stage),
            ])
        }
    }

    mod roundtrip {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// minicbor encodes and decodes Datum back to the same value.
            #[test]
            fn cbor(val: Datum) {
                let bytes = minicbor::to_vec(&val).unwrap();
                let recovered: Datum = minicbor::decode(&bytes).unwrap();
                prop_assert_eq!(val, recovered);
            }

            /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
            #[test]
            fn encoding_matches(val: Datum) {
                let mini = minicbor::to_vec(&val).unwrap();
                let pd = PlutusData::from(val).to_cbor();
                prop_assert_eq!(mini, pd);
            }

            /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
            #[test]
            fn from_plutus(val: Datum) {
                let pd_bytes = PlutusData::from(val.clone()).to_cbor();
                let recovered: Datum = minicbor::decode(&pd_bytes).unwrap();
                prop_assert_eq!(val, recovered);
            }

            /// From<Datum> for PlutusData and TryFrom<PlutusData> for Datum are mutual inverses.
            #[test]
            fn tryfrom(val: Datum) {
                let pd = PlutusData::from(val.clone());
                let recovered = Datum::try_from(pd).unwrap();
                prop_assert_eq!(val, recovered);
            }
        }
    }
}
