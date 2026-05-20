use crate::{Cheque, Squash, Unlocked, Unpend};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Redeemer {
    Defer,
    Main(Vec<Step>),
    Mutual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    Cont(Cont),
    Eol(Eol),
}

impl Step {
    pub fn is_adaptor(&self) -> bool {
        matches!(
            self,
            Step::Cont(Cont::Sub(_, _))
                | Step::Cont(Cont::Respond(_, _))
                | Step::Cont(Cont::Unlock(_))
        )
    }

    pub fn is_consumer(&self) -> bool {
        !self.is_adaptor()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cont {
    Add,
    Sub(Squash, Vec<Unlocked>),
    Close,
    Respond(Squash, Vec<Cheque>),
    Unlock(Vec<Unpend>),
    Expire(Vec<Unpend>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Eol {
    End,
    Elapse,
}

// =========================================================================
// minicbor Serialization
//
// All types use Plutus constructor encoding:
//   empty  constructor n → CBOR tag (121+n) + definite   array(0)
//   filled constructor n → CBOR tag (121+n) + indefinite array([fields...])
//
// Eol:
//   End    → tag 121 + array(0)
//   Elapse → tag 122 + array(0)
//
// Cont (6 variants, tags 121–126):
//   Add                      → tag 121 + array(0)
//   Sub(squash, unlockedVec) → tag 122 + indef-array [squash, list(unlocked)]
//   Close                    → tag 123 + array(0)
//   Respond(squash, cheques) → tag 124 + indef-array [squash, list(cheques)]
//   Unlock(unpends)          → tag 125 + indef-array [list(unpends)]
//   Expire(unpends)          → tag 126 + indef-array [list(unpends)]
//
// Step:
//   Cont(cont) → tag 121 + indef-array [cont]
//   Eol(eol)   → tag 122 + indef-array [eol]
//
// Redeemer:
//   Defer        → tag 121 + array(0)
//   Main(steps)  → tag 122 + indef-array [list(steps)]
//   Mutual       → tag 123 + array(0)
// =========================================================================

// --- Eol ---

impl<C> minicbor::Encode<C> for Eol {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Eol::End => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.array(0)?;
            }
            Eol::Elapse => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.array(0)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Eol {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        let result = match cbor_tag.as_u64() {
            121 => Eol::End,
            122 => Eol::Elapse,
            n => {
                return Err(minicbor::decode::Error::message(format!(
                    "unknown Eol CBOR tag {n}: expected 121 or 122"
                )));
            }
        };
        d.array()?;
        Ok(result)
    }
}

// --- Cont ---

impl<C> minicbor::Encode<C> for Cont
where
    Squash: minicbor::Encode<C>,
    Unlocked: minicbor::Encode<C>,
    Cheque: minicbor::Encode<C>,
    Unpend: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Cont::Add => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.array(0)?;
            }
            Cont::Sub(squash, unlocked) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?;
                e.encode_with(squash, ctx)?;
                crate::cbor_with::plutus_list::encode(unlocked, e, ctx)?;
                e.end()?;
            }
            Cont::Close => {
                e.tag(minicbor::data::Tag::new(123))?;
                e.array(0)?;
            }
            Cont::Respond(squash, cheques) => {
                e.tag(minicbor::data::Tag::new(124))?;
                e.begin_array()?;
                e.encode_with(squash, ctx)?;
                crate::cbor_with::plutus_list::encode(cheques, e, ctx)?;
                e.end()?;
            }
            Cont::Unlock(unpends) => {
                e.tag(minicbor::data::Tag::new(125))?;
                e.begin_array()?;
                crate::cbor_with::plutus_list::encode(unpends, e, ctx)?;
                e.end()?;
            }
            Cont::Expire(unpends) => {
                e.tag(minicbor::data::Tag::new(126))?;
                e.begin_array()?;
                crate::cbor_with::plutus_list::encode(unpends, e, ctx)?;
                e.end()?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Cont
where
    Squash: minicbor::Decode<'b, C>,
    Unlocked: minicbor::Decode<'b, C>,
    Cheque: minicbor::Decode<'b, C>,
    Unpend: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        match cbor_tag.as_u64() {
            121 => {
                d.array()?;
                Ok(Cont::Add)
            }
            122 => {
                d.array()?;
                let squash: Squash = d.decode_with(ctx)?;
                let unlocked: Vec<Unlocked> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Cont::Sub array",
                    ));
                }
                d.skip()?;
                Ok(Cont::Sub(squash, unlocked))
            }
            123 => {
                d.array()?;
                Ok(Cont::Close)
            }
            124 => {
                d.array()?;
                let squash: Squash = d.decode_with(ctx)?;
                let cheques: Vec<Cheque> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Cont::Respond array",
                    ));
                }
                d.skip()?;
                Ok(Cont::Respond(squash, cheques))
            }
            125 => {
                d.array()?;
                let unpends: Vec<Unpend> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Cont::Unlock array",
                    ));
                }
                d.skip()?;
                Ok(Cont::Unlock(unpends))
            }
            126 => {
                d.array()?;
                let unpends: Vec<Unpend> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Cont::Expire array",
                    ));
                }
                d.skip()?;
                Ok(Cont::Expire(unpends))
            }
            n => Err(minicbor::decode::Error::message(format!(
                "unknown Cont CBOR tag {n}: expected 121–126"
            ))),
        }
    }
}

// --- Step ---

impl<C> minicbor::Encode<C> for Step
where
    Cont: minicbor::Encode<C>,
    Eol: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Step::Cont(cont) => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.begin_array()?;
                e.encode_with(cont, ctx)?;
                e.end()?;
            }
            Step::Eol(eol) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?;
                e.encode_with(eol, ctx)?;
                e.end()?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Step
where
    Cont: minicbor::Decode<'b, C>,
    Eol: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        match cbor_tag.as_u64() {
            121 => {
                d.array()?;
                let cont: Cont = d.decode_with(ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Step::Cont array",
                    ));
                }
                d.skip()?;
                Ok(Step::Cont(cont))
            }
            122 => {
                d.array()?;
                let eol: Eol = d.decode_with(ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Step::Eol array",
                    ));
                }
                d.skip()?;
                Ok(Step::Eol(eol))
            }
            n => Err(minicbor::decode::Error::message(format!(
                "unknown Step CBOR tag {n}: expected 121 or 122"
            ))),
        }
    }
}

// --- Redeemer ---

impl<C> minicbor::Encode<C> for Redeemer
where
    Step: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Redeemer::Defer => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.array(0)?;
            }
            Redeemer::Main(steps) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?;
                crate::cbor_with::plutus_list::encode(steps, e, ctx)?;
                e.end()?;
            }
            Redeemer::Mutual => {
                e.tag(minicbor::data::Tag::new(123))?;
                e.array(0)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Redeemer
where
    Step: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        match cbor_tag.as_u64() {
            121 => {
                d.array()?;
                Ok(Redeemer::Defer)
            }
            122 => {
                d.array()?;
                let steps: Vec<Step> = crate::cbor_with::plutus_list::decode(d, ctx)?;
                if d.datatype()? != minicbor::data::Type::Break {
                    return Err(minicbor::decode::Error::message(
                        "expected end of Redeemer::Main array",
                    ));
                }
                d.skip()?;
                Ok(Redeemer::Main(steps))
            }
            123 => {
                d.array()?;
                Ok(Redeemer::Mutual)
            }
            n => Err(minicbor::decode::Error::message(format!(
                "unknown Redeemer CBOR tag {n}: expected 121, 122, or 123"
            ))),
        }
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Eol {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![Just(Eol::End), Just(Eol::Elapse)].boxed()
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Cont {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            Just(Cont::Add),
            (
                any::<Squash>(),
                proptest::collection::vec(any::<Unlocked>(), 0..=crate::MAX_UNSQUASHED)
            )
                .prop_map(|(s, u)| Cont::Sub(s, u)),
            Just(Cont::Close),
            (
                any::<Squash>(),
                proptest::collection::vec(any::<Cheque>(), 0..=crate::MAX_UNSQUASHED)
            )
                .prop_map(|(s, c)| Cont::Respond(s, c)),
            proptest::collection::vec(any::<Unpend>(), 0..=crate::MAX_UNSQUASHED)
                .prop_map(Cont::Unlock),
            proptest::collection::vec(any::<Unpend>(), 0..=crate::MAX_UNSQUASHED)
                .prop_map(Cont::Expire),
        ]
        .boxed()
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Step {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            any::<Cont>().prop_map(Step::Cont),
            any::<Eol>().prop_map(Step::Eol),
        ]
        .boxed()
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Redeemer {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            Just(Redeemer::Defer),
            proptest::collection::vec(any::<Step>(), 0..=crate::MAX_UNSQUASHED)
                .prop_map(Redeemer::Main),
            Just(Redeemer::Mutual),
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

    // --- Eol ---

    impl<'a> TryFrom<&PlutusData<'a>> for Eol {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (tag, _fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
            match tag {
                0 => Ok(Eol::End),
                1 => Ok(Eol::Elapse),
                _ => Err(anyhow!("Unknown Eol tag: {}", tag)),
            }
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Eol {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Eol::try_from(&data)
        }
    }

    impl<'a> From<Eol> for PlutusData<'a> {
        fn from(value: Eol) -> Self {
            match value {
                Eol::End => constr!(0),
                Eol::Elapse => constr!(1),
            }
        }
    }

    // --- Cont ---

    impl<'a> TryFrom<&PlutusData<'a>> for Cont {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
            match tag {
                0 => Ok(Cont::Add),
                1 => {
                    let [a, b] = <[PlutusData; 2]>::try_from(fields.collect::<Vec<_>>())
                        .map_err(|_| anyhow!("invalid 'Cont::Sub'"))?;
                    let squash = Squash::try_from(a)?;
                    let unlocked = <Vec<PlutusData>>::try_from(&b)?
                        .into_iter()
                        .map(|x| Unlocked::try_from(&x))
                        .collect::<anyhow::Result<Vec<Unlocked>>>()?;
                    Ok(Cont::Sub(squash, unlocked))
                }
                2 => Ok(Cont::Close),
                3 => {
                    let [a, b] = <[PlutusData; 2]>::try_from(fields.collect::<Vec<_>>())
                        .map_err(|_| anyhow!("invalid 'Cont::Respond'"))?;
                    let squash = Squash::try_from(a)?;
                    let cheques = <Vec<PlutusData>>::try_from(&b)?
                        .into_iter()
                        .map(Cheque::try_from)
                        .collect::<anyhow::Result<Vec<Cheque>>>()?;
                    Ok(Cont::Respond(squash, cheques))
                }
                4 => {
                    let [a] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                        .map_err(|_| anyhow!("invalid 'Cont::Unlock'"))?;
                    let unpends = <Vec<PlutusData>>::try_from(&a)?
                        .into_iter()
                        .map(Unpend::try_from)
                        .collect::<anyhow::Result<Vec<Unpend>>>()?;
                    Ok(Cont::Unlock(unpends))
                }
                5 => {
                    let [field] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                        .map_err(|_| anyhow!("invalid 'Cont::Expire'"))?;
                    let unpends = <Vec<PlutusData>>::try_from(&field)?
                        .into_iter()
                        .map(Unpend::try_from)
                        .collect::<anyhow::Result<Vec<Unpend>>>()?;
                    Ok(Cont::Expire(unpends))
                }
                _ => Err(anyhow!("Unknown Cont tag: {}", tag)),
            }
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Cont {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Cont::try_from(&data)
        }
    }

    impl<'a> From<Cont> for PlutusData<'a> {
        fn from(value: Cont) -> Self {
            match value {
                Cont::Add => constr!(0),
                Cont::Sub(squash, unlocked) => constr!(
                    1,
                    PlutusData::from(squash),
                    PlutusData::list(
                        unlocked
                            .into_iter()
                            .map(PlutusData::from)
                            .collect::<Vec<PlutusData>>(),
                    )
                ),
                Cont::Close => constr!(2),
                Cont::Respond(squash, cheques) => constr!(
                    3,
                    PlutusData::from(squash),
                    PlutusData::list(
                        cheques
                            .into_iter()
                            .map(PlutusData::from)
                            .collect::<Vec<PlutusData>>(),
                    ),
                ),
                Cont::Unlock(unpends) => constr!(
                    4,
                    PlutusData::list(
                        unpends
                            .into_iter()
                            .map(PlutusData::from)
                            .collect::<Vec<PlutusData>>(),
                    ),
                ),
                Cont::Expire(unpends) => constr!(
                    5,
                    PlutusData::list(
                        unpends
                            .into_iter()
                            .map(PlutusData::from)
                            .collect::<Vec<PlutusData>>(),
                    ),
                ),
            }
        }
    }

    // --- Step ---

    impl<'a> TryFrom<&PlutusData<'a>> for Step {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
            let [field] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                .map_err(|_| anyhow!("invalid 'Step'"))?;
            match tag {
                0 => Ok(Step::Cont(Cont::try_from(field)?)),
                1 => Ok(Step::Eol(Eol::try_from(field)?)),
                _ => Err(anyhow!("Unknown Step tag: {}", tag)),
            }
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Step {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Step::try_from(&data)
        }
    }

    impl<'a> From<Step> for PlutusData<'a> {
        fn from(value: Step) -> Self {
            match value {
                Step::Cont(cont) => constr!(0, PlutusData::from(cont)),
                Step::Eol(eol) => constr!(1, PlutusData::from(eol)),
            }
        }
    }

    // --- Redeemer ---

    impl<'a> TryFrom<&PlutusData<'a>> for Redeemer {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
            match tag {
                0 => Ok(Redeemer::Defer),
                1 => {
                    let [steps_pd] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                        .map_err(|_| anyhow!("invalid 'Redeemer::Main'"))?;
                    let steps = <Vec<PlutusData>>::try_from(&steps_pd)?
                        .into_iter()
                        .map(Step::try_from)
                        .collect::<anyhow::Result<Vec<Step>>>()?;
                    Ok(Redeemer::Main(steps))
                }
                2 => Ok(Redeemer::Mutual),
                _ => Err(anyhow!("Unknown Redeemer tag: {}", tag)),
            }
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Redeemer {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Redeemer::try_from(&data)
        }
    }

    impl<'a> From<Redeemer> for PlutusData<'a> {
        fn from(value: Redeemer) -> Self {
            match value {
                Redeemer::Defer => constr!(0),
                Redeemer::Main(steps) => constr!(
                    1,
                    PlutusData::list(
                        steps
                            .into_iter()
                            .map(PlutusData::from)
                            .collect::<Vec<PlutusData>>(),
                    ),
                ),
                Redeemer::Mutual => constr!(2),
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
    // ---- Eol ----

    /// minicbor encodes and decodes Eol back to the same value.
    #[test]
    fn eol_cbor(val: Eol) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Eol = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn eol_encoding_matches(val: Eol) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn eol_from_plutus(val: Eol) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Eol = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Eol> for PlutusData and TryFrom<PlutusData> for Eol are mutual inverses.
        #[test]
        fn eol_tryfrom(val: Eol) {
            let pd = PlutusData::from(val.clone());
            let recovered = Eol::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }

        // ---- Cont ----

        /// minicbor encodes and decodes Cont back to the same value.
        #[test]
        fn cont_cbor(val: Cont) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Cont = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn cont_encoding_matches(val: Cont) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn cont_from_plutus(val: Cont) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Cont = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Cont> for PlutusData and TryFrom<PlutusData> for Cont are mutual inverses.
        #[test]
        fn cont_tryfrom(val: Cont) {
            let pd = PlutusData::from(val.clone());
            let recovered = Cont::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }

        // ---- Step ----

        /// minicbor encodes and decodes Step back to the same value.
        #[test]
        fn step_cbor(val: Step) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Step = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn step_encoding_matches(val: Step) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn step_from_plutus(val: Step) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Step = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Step> for PlutusData and TryFrom<PlutusData> for Step are mutual inverses.
        #[test]
        fn step_tryfrom(val: Step) {
            let pd = PlutusData::from(val.clone());
            let recovered = Step::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }

        // ---- Redeemer ----

        /// minicbor encodes and decodes Redeemer back to the same value.
        #[test]
        fn redeemer_cbor(val: Redeemer) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Redeemer = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn redeemer_encoding_matches(val: Redeemer) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn redeemer_from_plutus(val: Redeemer) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Redeemer = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Redeemer> for PlutusData and TryFrom<PlutusData> for Redeemer are mutual inverses.
        #[test]
        fn redeemer_tryfrom(val: Redeemer) {
            let pd = PlutusData::from(val.clone());
            let recovered = Redeemer::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
