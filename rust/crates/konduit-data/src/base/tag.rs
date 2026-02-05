use core::fmt;

use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Tag(pub Vec<u8>);

impl std::str::FromStr for Tag {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Tag(
            hex::decode(s).map_err(|e| anyhow!(e).context("invalid tag"))?
        ))
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

impl From<Vec<u8>> for Tag {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

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
