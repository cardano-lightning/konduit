use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, PlutusDataDecodeError, VerificationKey};
use serde::{Deserialize, Serialize};

use crate::{Tag, impl_hex_serde_for_wrapper};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Keytag(pub Vec<u8>);

impl Keytag {
    pub fn new(key: VerificationKey, tag: Tag) -> Self {
        Self(
            key.0
                .as_ref()
                .to_vec()
                .into_iter()
                .chain(tag.0.clone())
                .collect::<Vec<u8>>(),
        )
    }

    pub fn split(&self) -> (VerificationKey, Tag) {
        (
            VerificationKey::from(<[u8; 32]>::try_from(self.0[0..32].to_vec()).unwrap()),
            Tag(self.0[32..].to_vec()),
        )
    }
}

impl_hex_serde_for_wrapper!(Keytag, Vec<u8>);

impl std::str::FromStr for Keytag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Keytag(
            hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Keytag {
    type Error = PlutusDataDecodeError;

    fn try_from(data: &PlutusData<'a>) -> Result<Self, Self::Error> {
        Ok(Self(<&'_ [u8]>::try_from(data)?.to_vec()))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Keytag {
    type Error = PlutusDataDecodeError;

    fn try_from(data: PlutusData<'a>) -> Result<Self, Self::Error> {
        Self::try_from(&data)
    }
}

impl<'a> From<Keytag> for PlutusData<'a> {
    fn from(value: Keytag) -> Self {
        Self::bytes(value.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_vec_serializes_to_hex_string() {
        let data = Keytag(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, r#""deadbeef""#);
        let data_simple = Keytag(vec![0x01, 0x02, 0x03, 0xFF]);
        let serialized_simple = serde_json::to_string(&data_simple).unwrap();
        assert_eq!(serialized_simple, r#""010203ff""#);
        let data_empty = Keytag(vec![]);
        let serialized_empty = serde_json::to_string(&data_empty).unwrap();
        assert_eq!(serialized_empty, r#""""#);
    }

    #[test]
    fn test_hex_vec_deserializes_from_hex_string() {
        let expected_data = Keytag(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let json_data = r#""deadbeef""#;
        let deserialized: Keytag = serde_json::from_str(json_data).unwrap();
        assert_eq!(deserialized, expected_data);
        let expected_simple = Keytag(vec![0x01, 0x02, 0x03, 0xFF]);
        let json_simple = r#""010203ff""#;
        let deserialized_simple: Keytag = serde_json::from_str(json_simple).unwrap();
        assert_eq!(deserialized_simple, expected_simple);
        let expected_empty = Keytag(vec![]);
        let json_empty = r#""""#;
        let deserialized_empty: Keytag = serde_json::from_str(json_empty).unwrap();
        assert_eq!(deserialized_empty, expected_empty);
    }

    #[test]
    fn test_hex_vec_deserialize_error_cases() {
        let json_invalid_chars = r#""nothex""#;
        let result = serde_json::from_str::<Keytag>(json_invalid_chars);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("failed to decode hex string: Invalid character")
        );
        let json_invalid_length = r#""123""#;
        let result_len = serde_json::from_str::<Keytag>(json_invalid_length);
        assert!(result_len.is_err());
        assert!(
            result_len
                .unwrap_err()
                .to_string()
                .contains("failed to decode hex string: Odd number of digits")
        );
    }
}
