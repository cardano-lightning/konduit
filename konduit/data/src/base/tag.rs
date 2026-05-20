use crate::ParseError;
use minicbor::{Decode, Encode, Encoder};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{convert::Infallible, fmt, ops::Deref, str::FromStr};

#[serde_as]
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Tag(
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(with = "crate::cbor_with::plutus_bytes", n(0))]
    Vec<u8>,
);

impl Tag {
    pub fn generate(length: usize) -> Self {
        let mut bytes = vec![0; length];
        rand_core::OsRng.fill_bytes(&mut bytes);
        Self::from(bytes)
    }

    pub fn data_result<T: Encode<()>>(
        &self,
        data: T,
    ) -> Result<Vec<u8>, minicbor::encode::Error<Infallible>> {
        let mut encoder = Encoder::new(Vec::new());
        encoder.begin_array()?.encode(self)?.encode(data)?.end()?;
        Ok(encoder.into_writer())
    }

    /// Tag data!
    pub fn data<T: Encode<()>>(&self, data: T) -> Vec<u8> {
        self.data_result(data)
            .expect("Cbor encode should be infalliable")
    }
}

impl FromStr for Tag {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(Tag(hex::decode(s)?))
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0.clone()))
    }
}

impl AsRef<[u8]> for Tag {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for Tag {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<u8>> for Tag {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for Tag {
    fn from(value: &[u8]) -> Self {
        Self(Vec::from(value))
    }
}

impl From<Tag> for Vec<u8> {
    fn from(value: Tag) -> Self {
        value.0
    }
}

impl From<&Tag> for Vec<u8> {
    fn from(value: &Tag) -> Self {
        value.0.clone()
    }
}

#[cfg(feature = "cddl")]
impl cuddly::ToCddl for Tag {
    fn cddl_ref() -> String {
        "tag".to_string()
    }
    fn cddl_definition() -> Option<String> {
        Some("tag = bytes".to_string())
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Tag {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let tag = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid tag"))?;
            Ok(Self(Vec::from(tag)))
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Tag {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Tag> for PlutusData<'a> {
        fn from(value: Tag) -> Self {
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
    fn tag_encodes_as_plutus_bytes() {
        let raw = vec![0xde, 0xad, 0xbe, 0xef];
        let tag = Tag(raw.clone());
        let mini_bytes = minicbor::to_vec(&tag).unwrap();
        let pd_bytes = PlutusData::bytes(raw).to_cbor();
        assert_eq!(mini_bytes, pd_bytes);
    }

    proptest! {
        #[test]
        fn tag_cbor_roundtrip(tag: Tag) {
            let bytes = minicbor::to_vec(&tag).unwrap();
            let recovered: Tag = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(tag, recovered);
        }

        #[test]
        fn tag_matches_plutus_encoding(raw: Vec<u8>) {
            let tag = Tag(raw.clone());
            let mini_bytes = minicbor::to_vec(&tag).unwrap();
            let pd_bytes = PlutusData::bytes(raw).to_cbor();
            prop_assert_eq!(mini_bytes, pd_bytes);
        }

        #[test]
        fn tag_plutus_roundtrip(tag: Tag) {
            let pd = PlutusData::bytes(tag.0.clone());
            let recovered: Tag = minicbor::decode(&pd.to_cbor()).unwrap();
            prop_assert_eq!(tag, recovered);
        }
    }
}
