use crate::{ParseError, Tag, VerifyingKey};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;

#[serde_as]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Keytag(
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(with = "crate::cbor_with::plutus_bytes", n(0))]
    Vec<u8>,
);

impl AsRef<[u8]> for Keytag {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Keytag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0.clone()))
    }
}

impl TryFrom<Vec<u8>> for Keytag {
    type Error = ParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, ParseError> {
        if value.len() < 32 {
            return Err(ParseError::Constraint(
                "invalid length for keytag; must be at least 32 bytes".to_string(),
            ));
        }
        Ok(Self(value))
    }
}

impl Keytag {
    /// Construct a Keytag by concatenating a VerifyingKey and a Tag.
    pub fn new(key: VerifyingKey, tag: Tag) -> Self {
        Self(key.as_ref().iter().chain(tag.as_ref()).copied().collect())
    }

    /// Split back into the constituent VerifyingKey and Tag.
    pub fn split(&self) -> (VerifyingKey, Tag) {
        (
            VerifyingKey::from_bytes(<[u8; 32]>::try_from(&self.0[..32]).unwrap()),
            Tag::from(self.0[32..].to_vec()),
        )
    }
}

impl std::str::FromStr for Keytag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(Keytag(hex::decode(s)?))
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Keytag {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        proptest::collection::vec(any::<u8>(), 32..=64)
            .prop_map(|v| Keytag::try_from(v).unwrap())
            .boxed()
    }
}

#[cfg(feature = "cddl")]
impl cuddly::ToCddl for Keytag {
    fn cddl_ref() -> String {
        "keytag".to_string()
    }
    fn cddl_definition() -> Option<String> {
        Some("keytag = bytes".to_string())
    }
}

// =========================================================================
// PlutusData Conversions (cardano_sdk-gated)
// =========================================================================
#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Keytag {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let raw = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid keytag"))?;
            Self::try_from(raw.to_vec()).map_err(Into::into)
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Keytag {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Keytag> for PlutusData<'a> {
        fn from(value: Keytag) -> Self {
            Self::bytes(value.0)
        }
    }
}

#[cfg(feature = "proptest")]
#[allow(unused_imports)]
mod roundtrip {
    use super::*;
    use cardano_sdk::{PlutusData, cbor::ToCbor};
    use proptest::prelude::*;

    #[test]
    fn keytag_encodes_as_plutus_bytes() {
        let raw = vec![0xabu8; 36]; // 32-byte key + 4-byte tag
        let keytag = Keytag::try_from(raw.clone()).unwrap();
        let mini_bytes = minicbor::to_vec(&keytag).unwrap();
        let pd_bytes = ToCbor::to_cbor(&PlutusData::bytes(raw));
        assert_eq!(mini_bytes, pd_bytes);
    }

    proptest! {
        /// minicbor encodes and decodes Keytag back to the same value.
        #[test]
        fn keytag_cbor_roundtrip(val: Keytag) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Keytag = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn keytag_matches_plutus_encoding(val: Keytag) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn keytag_plutus_roundtrip(val: Keytag) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Keytag = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Keytag> for PlutusData and TryFrom<PlutusData> for Keytag are mutual inverses.
        #[test]
        fn keytag_plutus_tryfrom_roundtrip(val: Keytag) {
            let pd = PlutusData::from(val.clone());
            let recovered = Keytag::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
