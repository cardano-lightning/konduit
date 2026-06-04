use crate::{ParseError, Secret, utils::try_into_array};
use cryptoxide::hashing::sha256;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;

#[serde_as]
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Lock(#[serde_as(as = "serde_with::hex::Hex")] pub [u8; 32]);

impl fmt::Display for Lock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(&hex::encode(self.0))
    }
}

impl std::str::FromStr for Lock {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(Lock(try_into_array(&hex::decode(s)?)?))
    }
}

impl AsRef<[u8]> for Lock {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for Lock {
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(<[u8; 32]>::try_from(value)?))
    }
}

impl From<[u8; 32]> for Lock {
    fn from(hash: [u8; 32]) -> Self {
        Lock(hash)
    }
}

impl From<Secret> for Lock {
    fn from(value: Secret) -> Self {
        Lock(sha256(&value.0))
    }
}

impl From<&Secret> for Lock {
    fn from(value: &Secret) -> Self {
        Lock(sha256(&value.0))
    }
}

impl<C> minicbor::Encode<C> for Lock {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Lock {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let raw = d.bytes()?;
        <[u8; 32]>::try_from(raw)
            .map(Self)
            .map_err(|_| minicbor::decode::Error::message("lock must be exactly 32 bytes"))
    }
}

#[test]
fn verify_encoding_differs_from_default_array() {
    let raw = [0xabu8; 32];

    let our_encoding = minicbor::to_vec(&Lock(raw)).unwrap();

    let mut e = minicbor::Encoder::new(Vec::new());
    e.array(32).unwrap();
    for b in &raw {
        e.u8(*b).unwrap();
    }
    let array_encoding = e.into_writer();

    assert_ne!(our_encoding, array_encoding);
    assert_eq!(our_encoding[0], 0x58); // CBOR bytes, 1-byte length
    assert_eq!(our_encoding[1], 0x20); // length = 32
    assert_eq!(array_encoding[0], 0x98); // CBOR array
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Lock {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let v = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid lock"))?;
            Ok(Self(try_into_array(v)?))
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Lock {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Lock> for PlutusData<'a> {
        fn from(value: Lock) -> Self {
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
    fn lock_encodes_as_plutus_bytes() {
        let raw = [0xabu8; 32];
        let lock = Lock(raw);
        let mini_bytes = minicbor::to_vec(&lock).unwrap();
        let pd_bytes = ToCbor::to_cbor(&PlutusData::bytes(raw));
        assert_eq!(mini_bytes, pd_bytes);
    }

    proptest! {
        /// minicbor encodes and decodes Lock back to the same value.
        #[test]
        fn lock_cbor_roundtrip(val: Lock) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Lock = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn lock_matches_plutus_encoding(val: Lock) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn lock_plutus_roundtrip(val: Lock) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Lock = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Lock> for PlutusData and TryFrom<PlutusData> for Lock are mutual inverses.
        #[test]
        fn lock_plutus_tryfrom_roundtrip(val: Lock) {
            let pd = PlutusData::from(val.clone());
            let recovered = Lock::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
