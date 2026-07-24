use crate::{ParseError, Secret, utils::try_into_array};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// An unpending instruction — either continue holding, expire, or unlock with a secret.
///
/// On-chain encoding: an empty bytestring for `Continue`, a 1-byte bytestring for
/// `Expire`, and a 32-byte bytestring for `Unlock`. Any non-zero, non-32-byte length
/// is treated as `Expire` when decoding.
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Unpend {
    Continue,
    Expire,
    Unlock(
        #[cfg_attr(
            feature = "serde",
            serde(with = "serde_with::As::<serde_with::hex::Hex>")
        )]
        [u8; 32],
    ),
}

impl Unpend {
    pub fn is_continue(&self) -> bool {
        matches!(self, Unpend::Continue)
    }
}

impl From<&Secret> for Unpend {
    fn from(value: &Secret) -> Self {
        Self::Unlock(value.0)
    }
}

impl std::str::FromStr for Unpend {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            return Ok(Unpend::Continue);
        }
        Ok(Unpend::Unlock(try_into_array(&hex::decode(s)?)?))
    }
}

impl<C> minicbor::Encode<C> for Unpend {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Unpend::Continue => e.bytes(b"")?,
            Unpend::Expire => e.bytes(&[0])?,
            Unpend::Unlock(arr) => e.bytes(arr.as_slice())?,
        };
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Unpend {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let raw = d.bytes()?;
        Ok(match raw.len() {
            0 => Unpend::Continue,
            32 => Unpend::Unlock(
                <[u8; 32]>::try_from(raw)
                    .map_err(|_| minicbor::decode::Error::message("expected 32 bytes"))?,
            ),
            _ => Unpend::Expire,
        })
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use cardano_sdk::PlutusData;

    impl<'a> TryFrom<&PlutusData<'a>> for Unpend {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let bytes = <&[u8]>::try_from(data).map_err(|e| e.context("invalid unpend"))?;
            Ok(match bytes.len() {
                0 => Unpend::Continue,
                32 => Unpend::Unlock(<[u8; 32]>::try_from(bytes)?),
                _ => Unpend::Expire,
            })
        }
    }

    impl<'a> TryFrom<PlutusData<'a>> for Unpend {
        type Error = anyhow::Error;

        fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
            Self::try_from(&data)
        }
    }

    impl<'a> From<Unpend> for PlutusData<'a> {
        fn from(value: Unpend) -> Self {
            match value {
                Unpend::Continue => PlutusData::bytes([]),
                Unpend::Expire => PlutusData::bytes([0]),
                Unpend::Unlock(arr) => PlutusData::bytes(arr),
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

    #[test]
    fn unpend_variants_encode_correctly() {
        // Continue → empty bytes (CBOR: 0x40)
        assert_eq!(
            minicbor::to_vec(&Unpend::Continue).unwrap(),
            ToCbor::to_cbor(&PlutusData::bytes([] as [u8; 0]))
        );
        // Expire → 1 byte (CBOR: 0x41 0x00)
        assert_eq!(
            minicbor::to_vec(&Unpend::Expire).unwrap(),
            ToCbor::to_cbor(&PlutusData::bytes([0u8]))
        );
        // Unlock → 32 bytes
        let key = [0xabu8; 32];
        assert_eq!(
            minicbor::to_vec(&Unpend::Unlock(key)).unwrap(),
            ToCbor::to_cbor(&PlutusData::bytes(key))
        );
    }

    proptest! {
        /// minicbor encodes and decodes Unpend back to the same value.
        #[test]
        fn unpend_cbor_roundtrip(val: Unpend) {
            let bytes = minicbor::to_vec(&val).unwrap();
            let recovered: Unpend = minicbor::decode(&bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// minicbor bytes are byte-for-byte identical to PlutusData's canonical CBOR.
        #[test]
        fn unpend_matches_plutus_encoding(val: Unpend) {
            let mini = minicbor::to_vec(&val).unwrap();
            let pd = PlutusData::from(val).to_cbor();
            prop_assert_eq!(mini, pd);
        }

        /// PlutusData's canonical CBOR decodes via minicbor back to the same value.
        #[test]
        fn unpend_plutus_roundtrip(val: Unpend) {
            let pd_bytes = PlutusData::from(val.clone()).to_cbor();
            let recovered: Unpend = minicbor::decode(&pd_bytes).unwrap();
            prop_assert_eq!(val, recovered);
        }

        /// From<Unpend> for PlutusData and TryFrom<PlutusData> for Unpend are mutual inverses.
        #[test]
        fn unpend_plutus_tryfrom_roundtrip(val: Unpend) {
            let pd = PlutusData::from(val.clone());
            let recovered = Unpend::try_from(pd).unwrap();
            prop_assert_eq!(val, recovered);
        }
    }
}
