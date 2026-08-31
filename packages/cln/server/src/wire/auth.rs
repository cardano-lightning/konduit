//! Wire-level auth: `Auth` is the trait any header-carried credential
//! implements, resolving to the `(VerifyingKey, Tag)`
//! the defactor identity of inbonud channels.

use konduit_data::{Tag, VerifyingKey};
use std::str::FromStr;

pub trait Auth: Sized {
    type Err;
    fn parse(header: &str) -> Result<Self, Self::Err>;
    fn resolve(&self) -> Result<(VerifyingKey, Tag), Self::Err>;
}

pub const HEADER: &str = "konduit";
const KEY_LEN: usize = 32;

/// Keytag is the simplest possible example.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Keytag(Vec<u8>);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not valid hex")]
    Hex,
    #[error("too short for a verifying key")]
    TooShort,
    #[error("invalid verifying key")]
    BadKey,
}

impl Keytag {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl From<(&VerifyingKey, &Tag)> for Keytag {
    fn from((key, tag): (&VerifyingKey, &Tag)) -> Self {
        let mut bytes = <[u8; 32]>::from(key.clone()).to_vec();
        bytes.extend_from_slice(tag.as_ref());
        Self(bytes)
    }
}

impl TryFrom<Keytag> for (VerifyingKey, Tag) {
    type Error = Error;
    fn try_from(kt: Keytag) -> Result<Self, Self::Error> {
        if kt.0.len() < KEY_LEN {
            return Err(Error::TooShort);
        }
        let (key_bytes, tag_bytes) = kt.0.split_at(KEY_LEN);
        let key = VerifyingKey::try_from(key_bytes).map_err(|_| Error::BadKey)?;
        Ok((key, Tag::from(tag_bytes.to_vec())))
    }
}

impl FromStr for Keytag {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex::decode(s).map(Self).map_err(|_| Error::Hex)
    }
}

impl Auth for Keytag {
    type Err = Error;
    fn parse(header: &str) -> Result<Self, Self::Err> {
        header.parse()
    }
    fn resolve(&self) -> Result<(VerifyingKey, Tag), Self::Err> {
        self.clone().try_into()
    }
}
