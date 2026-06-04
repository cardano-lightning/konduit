use crate::{ParseError, utils::try_into_array};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Secret(#[serde_as(as = "serde_with::hex::Hex")] pub [u8; 32]);

impl std::str::FromStr for Secret {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(Secret(try_into_array(&hex::decode(s)?)?))
    }
}

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for Secret {
    type Error = ParseError;

    fn try_from(value: Vec<u8>) -> Result<Self, ParseError> {
        let len = value.len();
        Ok(Self(<[u8; 32]>::try_from(value).map_err(|_| {
            ParseError::WrongLength {
                expected: 32,
                got: len,
            }
        })?))
    }
}

impl<C> minicbor::Encode<C> for Secret {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Secret {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let raw = d.bytes()?;
        <[u8; 32]>::try_from(raw)
            .map(Self)
            .map_err(|_| minicbor::decode::Error::message("secret must be exactly 32 bytes"))
    }
}

#[test]
fn verify_encoding_differs_from_default_array() {
    let raw = [0xdeu8; 32];

    // What our Encode impl produces (bytes encoding, matching PlutusData)
    let our_encoding = minicbor::to_vec(&Secret(raw)).unwrap();

    // What minicbor would produce if [u8;32] were encoded as a CBOR array of integers (the default for arrays)
    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(32).unwrap();
    for b in &raw {
        e.u8(*b).unwrap();
    }
    let array_encoding = e.into_writer();

    // They must differ — proving the manual impl is necessary
    assert_ne!(
        our_encoding, array_encoding,
        "encoding should NOT match default array-of-ints"
    );
    // And our encoding is compact bytes: 0x58 0x20 <32 bytes>
    assert_eq!(our_encoding[0], 0x58); // major type 2 (bytes), 1-byte length follows
    assert_eq!(our_encoding[1], 0x20); // length = 32
    // While array-of-ints starts with 0x98 0x20 (major type 4, 32 items)
    assert_eq!(array_encoding[0], 0x98); // major type 4 (array)
    assert_eq!(array_encoding[1], 0x20); // 32 items
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Secret {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let v = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid secret"))?;
            Ok(Self(try_into_array(v)?))
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Secret {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Secret> for PlutusData<'a> {
        fn from(value: Secret) -> Self {
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
    fn secret_encodes_as_plutus_bytes() {
        let raw = [0u8; 32];
        let secret = Secret(raw);
        let mini_bytes = minicbor::to_vec(&secret).unwrap();
        let pd_bytes = ToCbor::to_cbor(&PlutusData::bytes(raw));
        assert_eq!(mini_bytes, pd_bytes);
    }

    proptest! {
        /// minicbor encodes and decodes Secret back to the same value.
        #[test]
        fn secret_cbor_roundtrip(val: Secret) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Secret = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn secret_matches_plutus_encoding(val: Secret) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn secret_plutus_roundtrip(val: Secret) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Secret = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Secret> for PlutusData and TryFrom<PlutusData> for Secret are mutual inverses.
        #[test]
        fn secret_plutus_tryfrom_roundtrip(val: Secret) {
            let pd = PlutusData::from(val.clone());
            let recovered = Secret::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
