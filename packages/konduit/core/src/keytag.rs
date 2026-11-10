use crate::{Tag, VerifyingKey};
use std::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Header carrying a channel identity on authenticated L2 requests.
pub const HEADER: &str = "KONDUIT";

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Keytag(#[cfg_attr(feature = "serde", serde(with = "crate::serde::bytes"))] Vec<u8>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid keytag length: expected at least 32 bytes, got {0}")]
pub struct InvalidKeytagLength(pub usize);

impl AsRef<[u8]> for Keytag {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Display for Keytag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl TryFrom<Vec<u8>> for Keytag {
    type Error = InvalidKeytagLength;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() < 32 {
            return Err(InvalidKeytagLength(value.len()));
        }
        Ok(Self(value))
    }
}

impl Keytag {
    pub fn new(key: &VerifyingKey, tag: &Tag) -> Self {
        Self::from_parts(key, tag)
    }

    pub fn from_parts(key: &VerifyingKey, tag: &Tag) -> Self {
        Self(
            <[u8; 32]>::from(*key)
                .into_iter()
                .chain(tag.as_ref().iter().copied())
                .collect(),
        )
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    pub fn split(&self) -> (VerifyingKey, Tag) {
        let key: [u8; 32] = self.0[..32]
            .try_into()
            .expect("Keytag invariant guarantees a 32-byte key prefix");
        (VerifyingKey::from(key), Tag::from(self.0[32..].to_vec()))
    }
}

impl std::str::FromStr for Keytag {
    type Err = InvalidKeytagLength;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(value).map_err(|_| InvalidKeytagLength(0))?;
        Self::try_from(bytes)
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::Keytag;
    use crate::{Tag, VerifyingKey};

    #[test]
    fn keytag_roundtrips_parts_and_hex_serde() {
        let key = VerifyingKey::from([0x11; 32]);
        let tag = Tag::from(vec![0x22; 8]);
        let keytag = Keytag::new(&key, &tag);
        assert_eq!(keytag.split(), (key, tag));

        let json = serde_json::to_string(&keytag).unwrap();
        assert_eq!(serde_json::from_str::<Keytag>(&json).unwrap(), keytag);
    }
}
