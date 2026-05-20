use crate::{
    ChequeSigned, Lock, SigningKey, Tag, Unverified, Verified, VerifyError, VerifyState,
    VerifyingKey, cheque_body::ChequeBody,
};

pub type Locked<V = Unverified> = ChequeSigned<Lock, V>;

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<V: VerifyState> Locked<V> {
    pub fn lock(&self) -> &Lock {
        self.latch()
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Locked<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified cheque and transitions it to `Locked<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerifyingKey,
        tag: &Tag,
    ) -> Result<Locked<Verified>, VerifyError> {
        if verification_key.verify(&tag.data(self.body()), &self.signature) {
            Ok(Locked::new_with_state(self.body, self.signature))
        } else {
            Err(VerifyError::InvalidSignature)
        }
    }
}

// =========================================================================
// Verified State Methods
// =========================================================================
impl Locked<Verified> {
    /// Signing a new cheque inherently guarantees its authenticity,
    /// so the constructor immediately returns a `Locked<Verified>` instance.
    pub fn make(signing_key: &SigningKey, tag: &Tag, body: ChequeBody) -> Self {
        let signature = signing_key.sign(&tag.data(&body));
        Self::new_with_state(body, signature)
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Locked<Unverified> {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use crate::Signature;
        use proptest::prelude::*;
        (any::<ChequeBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| Locked::new(body, Signature::from_bytes(sig_bytes)))
            .boxed()
    }
}

// =========================================================================
// PlutusData Conversions (cardano_sdk-gated)
// =========================================================================
#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Locked<Unverified> {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
            Self::try_from(fields)
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Locked<Unverified> {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            let fields: Vec<PlutusData<'_>> = Vec::try_from(&data)?;
            Self::try_from(fields)
        }
    }

    impl<'a> TryFrom<Vec<PlutusData<'a>>> for Locked<Unverified> {
        type Error = anyhow::Error;

        fn try_from(list: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
            let [a, b] = <[PlutusData; 2]>::try_from(list)
                .map_err(|_| anyhow::anyhow!("invalid 'Locked': expected 2-element list"))?;
            let sig_bytes: [u8; 64] = *<&[u8; 64]>::try_from(&b)?;
            Ok(Self::new(
                ChequeBody::try_from(a)?,
                crate::Signature::from_bytes(sig_bytes),
            ))
        }
    }

    impl<'a, V: VerifyState> From<Locked<V>> for PlutusData<'a> {
        fn from(locked: Locked<V>) -> Self {
            Self::list(vec![
                PlutusData::from(locked.body),
                PlutusData::bytes(locked.signature.to_bytes()),
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
        /// minicbor encodes and decodes Locked back to the same value.
        #[test]
        fn cbor(val: Locked<Unverified>) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Locked<Unverified> = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn encoding_matches(val: Locked<Unverified>) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn from_plutus(val: Locked<Unverified>) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Locked<Unverified> = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Locked> for PlutusData and TryFrom<PlutusData> for Locked are mutual inverses.
        #[test]
        fn tryfrom(val: Locked<Unverified>) {
            let pd = PlutusData::from(val.clone());
            let recovered = Locked::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
