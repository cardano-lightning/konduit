use crate::{
    ChequeBody, Duration, Lock, Secret, Signature, Tag, Unverified, Verified, VerifyError,
    VerifyState, VerifyingKey, locked::Locked, unlocked::Unlocked,
};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound(deserialize = "V: Default"))
)]
pub enum Cheque<V: VerifyState = Unverified> {
    Unlocked(Unlocked<V>),
    Locked(Locked<V>),
}

impl<V: VerifyState + Ord> PartialOrd for Cheque<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<V: VerifyState + Ord> Ord for Cheque<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.index().cmp(&other.index()) {
            ordering @ std::cmp::Ordering::Less | ordering @ std::cmp::Ordering::Greater => {
                ordering
            }
            std::cmp::Ordering::Equal => match (self, other) {
                (Self::Unlocked(a), Self::Unlocked(b)) => a.cmp(b),
                (Self::Locked(a), Self::Locked(b)) => a.cmp(b),
                (Self::Unlocked(_), _) => std::cmp::Ordering::Greater,
                (Self::Locked(_), _) => std::cmp::Ordering::Less,
            },
        }
    }
}

impl<V: VerifyState> Cheque<V> {
    pub fn is_cheque(&self) -> bool {
        !matches!(self, &Self::Unlocked(_))
    }

    pub fn body(&self) -> ChequeBody {
        match self {
            Self::Unlocked(unlocked) => unlocked.locked().body().clone(),
            Self::Locked(locked) => locked.body().clone(),
        }
    }

    pub fn index(&self) -> u64 {
        self.body().index()
    }

    pub fn amount(&self) -> u64 {
        self.body().amount()
    }

    pub fn lock(&self) -> Lock {
        *self.body().lock()
    }

    pub fn timeout(&self) -> Duration {
        self.body().timeout()
    }

    pub fn as_unlocked(&self) -> Option<Unlocked<V>> {
        match self {
            Cheque::Unlocked(unlocked) => Some(unlocked.clone()),
            Cheque::Locked(_) => None,
        }
    }

    pub fn as_locked(&self) -> Option<Locked<V>> {
        match self {
            Cheque::Unlocked(_) => None,
            Cheque::Locked(locked) => Some(locked.clone()),
        }
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Cheque<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified cheque and transitions it to `Cheque<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerifyingKey,
        tag: &Tag,
    ) -> Result<Cheque<Verified>, VerifyError> {
        match self {
            Cheque::Unlocked(x) => x.try_verify(verification_key, tag).map(Cheque::from),
            Cheque::Locked(x) => x.try_verify(verification_key, tag).map(Cheque::from),
        }
    }

    pub fn skip_verify(self) -> Cheque<Verified> {
        match self {
            Cheque::Unlocked(x) => Cheque::from(x.skip_verify()),
            Cheque::Locked(x) => Cheque::from(x.skip_verify()),
        }
    }
}

impl<V: VerifyState> From<Unlocked<V>> for Cheque<V> {
    fn from(value: Unlocked<V>) -> Self {
        Cheque::Unlocked(value)
    }
}

impl<V: VerifyState> From<Locked<V>> for Cheque<V> {
    fn from(value: Locked<V>) -> Self {
        Cheque::Locked(value)
    }
}

// =========================================================================
// Verified State Methods
// =========================================================================
impl Cheque<Verified> {
    pub fn into_unverified(self) -> Cheque<Unverified> {
        match self {
            Cheque::Unlocked(x) => Cheque::from(x.into_unverified()),
            Cheque::Locked(x) => Cheque::from(x.into_unverified()),
        }
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding:
//   Unlocked → CBOR tag 121 (constr 0) + indef-array [body, signature_bytes]
//   Locked   → CBOR tag 122 (constr 1) + indef-array [body, signature_bytes]
// =========================================================================
impl<C, V: VerifyState> minicbor::Encode<C> for Cheque<V>
where
    ChequeBody<Secret>: minicbor::Encode<C>,
    ChequeBody<Lock>: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Cheque::Unlocked(u) => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.begin_array()?;
                e.encode_with(&u.body, ctx)?;
                e.encode_with(u.signature, ctx)?;
                e.end()?;
            }
            Cheque::Locked(l) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?;
                e.encode_with(&l.body, ctx)?;
                e.encode_with(l.signature, ctx)?;
                e.end()?;
            }
        }
        Ok(())
    }
}

/// Decoding always produces `Cheque<Unverified>`.
impl<'b, C> minicbor::Decode<'b, C> for Cheque<Unverified>
where
    ChequeBody<Secret>: minicbor::Decode<'b, C>,
    ChequeBody<Lock>: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let cbor_tag = d.tag()?;
        let variant: u64 = match cbor_tag.as_u64() {
            121 => 0,
            122 => 1,
            n => {
                return Err(minicbor::decode::Error::message(format!(
                    "unknown Cheque CBOR tag {n}: expected 121 or 122"
                )));
            }
        };
        d.array()?;
        let result = match variant {
            0 => {
                let body: ChequeBody<Secret> = d.decode_with(ctx)?;
                let signature: Signature = d.decode_with(ctx)?;
                Cheque::Unlocked(Unlocked::new_with_state(body, signature))
            }
            1 => {
                let body: ChequeBody<Lock> = d.decode_with(ctx)?;
                let signature: Signature = d.decode_with(ctx)?;
                Cheque::Locked(Locked::new_with_state(body, signature))
            }
            _ => unreachable!(),
        };
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Cheque array",
            ));
        }
        d.skip()?;
        Ok(result)
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Cheque {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            any::<crate::Unlocked>().prop_map(Cheque::Unlocked),
            any::<crate::Locked>().prop_map(Cheque::Locked),
        ]
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
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<PlutusData<'a>> for Cheque {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let (variant, fields): (u64, Vec<PlutusData<'_>>) = (&data).try_into()?;
            match variant {
                0 => Unlocked::try_from(fields)
                    .map(Cheque::Unlocked)
                    .map_err(|e| e.context("invalid 'Unlocked' variant")),
                1 => Locked::try_from(fields)
                    .map(Cheque::Locked)
                    .map_err(|e| e.context("invalid 'Locked' variant")),
                _ => Err(anyhow!("unknown Cheque variant: {variant}")),
            }
        }
    }

    impl<'a, V: VerifyState> From<Cheque<V>> for PlutusData<'a> {
        fn from(value: Cheque<V>) -> Self {
            match value {
                Cheque::Unlocked(u) => PlutusData::constr(
                    0,
                    vec![
                        PlutusData::from(u.body),
                        PlutusData::bytes(u.signature.to_bytes()),
                    ],
                ),
                Cheque::Locked(l) => PlutusData::constr(
                    1,
                    vec![
                        PlutusData::from(l.body),
                        PlutusData::bytes(l.signature.to_bytes()),
                    ],
                ),
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
        /// minicbor encodes and decodes Cheque back to the same value.
        #[test]
        fn cbor(val: Cheque) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Cheque = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Cheque) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Cheque) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Cheque = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Cheque> for PlutusData and TryFrom<PlutusData> for Cheque are mutual inverses.
        #[test]
        fn tryfrom(val: Cheque) {
            let pd = PlutusData::from(val.clone());
            let recovered = Cheque::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
