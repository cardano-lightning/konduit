use anyhow::anyhow;
use cardano_sdk::PlutusData;
use rand_core::RngCore;
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Tag(Vec<u8>);

impl Tag {
    pub fn generate(length: usize) -> Self {
        let mut bytes = vec![0; length];
        rand_core::OsRng.fill_bytes(&mut bytes);
        Self::from(bytes)
    }
}

impl FromStr for Tag {
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

#[cfg(feature = "wasm")]
pub mod wasm {
    use std::{ops::Deref, str::FromStr};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    #[repr(transparent)]
    #[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
    pub struct Tag(super::Tag);

    impl Deref for Tag {
        type Target = super::Tag;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl From<Tag> for super::Tag {
        fn from(tag: Tag) -> Self {
            tag.0
        }
    }

    #[wasm_bindgen]
    impl Tag {
        #[wasm_bindgen(constructor)]
        pub fn new(value: &str) -> Result<Self, String> {
            super::Tag::from_str(value)
                .map_err(|e| e.to_string())
                .map(Self)
        }

        #[wasm_bindgen(constructor)]
        pub fn generate(length: usize) -> Self {
            Self(super::Tag::generate(length))
        }

        #[allow(clippy::inherent_to_string)]
        #[wasm_bindgen(js_name = "toString")]
        pub fn to_string(&self) -> String {
            self.deref().to_string()
        }
    }
}
