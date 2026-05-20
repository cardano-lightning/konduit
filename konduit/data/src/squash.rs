use std::marker::PhantomData;

use crate::{
    Indexes, Signature, SigningKey, SquashBody, Tag, Unverified, Verified, VerifyError,
    VerifyState, VerifyingKey,
};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Squash<V = Unverified> {
    #[cfg_attr(feature = "cddl", n(0))]
    body: SquashBody,
    #[cfg_attr(feature = "cddl", n(1))]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    signature: Signature,
    #[serde(skip)]
    #[cfg_attr(feature = "cddl", cddl(skip))]
    _marker: PhantomData<V>,
}

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<V> Squash<V> {
    /// Internal constructor to associate state markers.
    pub fn new_with_state(body: SquashBody, signature: Signature) -> Self {
        Self {
            body,
            signature,
            _marker: PhantomData,
        }
    }

    pub fn body(&self) -> &SquashBody {
        &self.body
    }

    pub fn index(&self) -> u64 {
        self.body.index()
    }

    pub fn amount(&self) -> u64 {
        self.body.amount()
    }

    pub fn exclude(&self) -> &Indexes {
        self.body.exclude()
    }

    pub fn is_index_squashed(&self, index: u64) -> bool {
        self.body.is_index_squashed(index)
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Squash<Unverified> {
    /// Creates a new, unverified squash from a raw body and signature.
    pub fn new(body: SquashBody, signature: Signature) -> Self {
        Self::new_with_state(body, signature)
    }

    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified squash and transitions it to `Squash<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerifyingKey,
        tag: &Tag,
    ) -> Result<Squash<Verified>, VerifyError> {
        if verification_key.verify(&tag.data(self.body()), &self.signature) {
            Ok(Squash::new_with_state(self.body, self.signature))
        } else {
            Err(VerifyError::InvalidSignature)
        }
    }

    /// The unsafe version. Suitable when the data comes from a trusted source,
    /// such as your own database.
    pub fn skip_verify(self) -> Squash<Verified> {
        Squash {
            body: self.body,
            signature: self.signature,
            _marker: PhantomData,
        }
    }
}

// =========================================================================
// Verified State Methods
// =========================================================================
impl Squash<Verified> {
    /// Signing a new squash inherently guarantees its authenticity,
    /// so the constructor immediately returns a `Squash<Verified>` instance.
    pub fn make(signing_key: &SigningKey, tag: &Tag, body: SquashBody) -> Self {
        let signature: Signature = signing_key.sign(&tag.data(&body));
        Self::new_with_state(body, signature)
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding: indefinite-length array of [body, signature_bytes].
// =========================================================================
impl<C, V: VerifyState> minicbor::Encode<C> for Squash<V> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(&self.body, ctx)?;
        e.encode_with(self.signature, ctx)?;
        e.end()?;
        Ok(())
    }
}

/// Decoding always produces `Squash<Unverified>` — verify explicitly with `try_verify`.
impl<'b, C> minicbor::Decode<'b, C> for Squash<Unverified> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let body: SquashBody = d.decode_with(ctx)?;
        let signature: Signature = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of Squash array",
            ));
        }
        d.skip()?;
        Ok(Self::new(body, signature))
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Squash {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<SquashBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| Squash::new(body, Signature::from_bytes(sig_bytes)))
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

    impl<'a> TryFrom<Vec<PlutusData<'a>>> for Squash {
        type Error = anyhow::Error;

        fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
            let [a, b] = <[PlutusData; 2]>::try_from(value)
                .map_err(|_| anyhow!("invalid 'Squash': expected 2-element list"))?;
            let sig_bytes: [u8; 64] = *<&[u8; 64]>::try_from(&b)?;
            Ok(Self::new(
                SquashBody::try_from(a)?,
                Signature::from_bytes(sig_bytes),
            ))
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Squash {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(Vec::try_from(&data)?)
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Squash<Verified> {
        type Error = anyhow::Error;
        fn try_from(_: PlutusData<'a>) -> anyhow::Result<Self> {
            anyhow::bail!(
                "Squash<Verified> cannot be decoded; decode as Squash<Unverified> and call try_verify"
            )
        }
    }

    impl<'a, V: VerifyState> From<Squash<V>> for PlutusData<'a> {
        fn from(value: Squash<V>) -> Self {
            Self::list(vec![
                PlutusData::from(&value.body),
                PlutusData::bytes(value.signature.to_bytes()),
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
        /// minicbor encodes and decodes Squash back to the same value.
        #[test]
        fn cbor(val: Squash) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Squash = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Squash) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Squash) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Squash = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Squash> for PlutusData and TryFrom<PlutusData> for Squash are mutual inverses.
        #[test]
        fn tryfrom(val: Squash) {
            let pd = PlutusData::from(val.clone());
            let recovered = Squash::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Indexes;

    use super::*;

    #[test]
    fn test_squash_round_trip() {
        let sk = SigningKey::from([0; 32]);
        let tag = Tag::from([1; 20].to_vec());
        let body = SquashBody::new_no_verify(120309, 123, Indexes::new([22].to_vec()).unwrap());
        let original = Squash::make(&sk, &tag, body);

        println!("{}", serde_json::to_string_pretty(&original).unwrap());
        let ser = serde_json::to_vec(&original).expect("Failed to serialize Squash");
        let de: Squash = serde_json::from_slice(&ser).expect("Failed to deserialize Squash");

        assert_eq!(original.body, de.body);
    }
}
