use konduit_data::{ParseError, Tag, VerifyingKey};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt;

#[serde_as]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[repr(transparent)]
pub struct Keytag(
    #[n(0)]
    #[serde_as(as = "serde_with::hex::Hex")]
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
