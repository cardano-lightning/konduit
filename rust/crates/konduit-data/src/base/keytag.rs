use std::fmt;

use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Tag;

#[serde_as]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Keytag(#[serde_as(as = "serde_with::hex::Hex")] pub Vec<u8>);

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
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl Keytag {
    pub fn new(key: VerificationKey, tag: Tag) -> Self {
        Self(
            key.as_ref()
                .to_vec()
                .into_iter()
                .chain(tag.as_ref().to_vec())
                .collect::<Vec<u8>>(),
        )
    }

    pub fn split(&self) -> (VerificationKey, Tag) {
        (
            VerificationKey::from(<[u8; 32]>::try_from(self.as_ref()[0..32].to_vec()).unwrap()),
            Tag::from(self.as_ref()[32..].to_vec()),
        )
    }
}

impl std::str::FromStr for Keytag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Keytag(
            hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?,
        ))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Keytag {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let tag = <&'_ [u8]>::try_from(data).map_err(|e| e.context("invalid tag"))?;
        Ok(Self(Vec::from(tag)))
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
