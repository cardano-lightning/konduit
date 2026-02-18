//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use anyhow::anyhow;
use std::{fmt, ops::Deref, str::FromStr};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// A _blake2b_ hash digest; typically 28 or 32 bytes long.
///
/// There are several ways to construct [`Self`], but fundamentally:
///
/// - Conversions from static byte arrays of known sizes are infaillible:
///
///   ```rust
///   # use cardano_tx_builder::Hash;
///   assert_eq!(
///     <Hash<28>>::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).to_string(),
///     "00000000000000000000000000000000000000000000000000000000",
///   );
///   ```
///
/// - Conversions from vectors or slices are possible but faillible:
///
///   ```rust
///   # use cardano_tx_builder::Hash;
///   // Vectors contains exactly 28 elements
///   assert!(
///     <Hash<28>>::try_from(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
///         .is_ok()
///   );
///
///   // Vectors still contains only 28 elements
///   assert!(
///     <Hash<32>>::try_from(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
///         .is_err()
///   );
///   ```
///
/// - Conversions from base16-encoded text strings are also possible:
///
///   ```rust
///   # use cardano_tx_builder::Hash;
///   // The text string is indeed 56 character-long.
///   assert!(
///     <Hash<28>>::try_from("00000000000000000000000000000000000000000000000000000000")
///         .is_ok()
///   );
///   ```
///
/// - For the latter, we also provide the [`hash!`](crate::hash) macro.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Hash<const SIZE: usize>(#[n(0)] pallas::Hash<SIZE>);

impl<const SIZE: usize> fmt::Display for Hash<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

// ---------------------------------------------------------------- Constructing

impl Hash<28> {
    pub fn new<T: AsRef<[u8]>>(preimage: T) -> Self {
        Self(pallas::Hasher::<224>::hash(preimage.as_ref()))
    }
}

impl Hash<32> {
    pub fn new<T: AsRef<[u8]>>(preimage: T) -> Self {
        Self(pallas::Hasher::<256>::hash(preimage.as_ref()))
    }
}

// ------------------------------------------------------------------ Inspecting

impl<const SIZE: usize> Hash<SIZE> {
    pub const SIZE: usize = SIZE;
}

// ----------------------------------------------------------- Converting (from)

impl<const SIZE: usize> TryFrom<&str> for Hash<SIZE> {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s).map_err(|e| anyhow!(e))?;
        let fixed_sized_bytes = <[u8; SIZE]>::try_from(bytes).map_err(|_| {
            anyhow!(
                "invalid hex string length; expected {}, got {}",
                2 * SIZE,
                s.len()
            )
        })?;

        Ok(Hash(pallas::Hash::new(fixed_sized_bytes)))
    }
}

impl<const SIZE: usize> FromStr for Hash<SIZE> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s).map_err(|e| e.to_string())
    }
}

impl<const SIZE: usize> TryFrom<Vec<u8>> for Hash<SIZE> {
    type Error = anyhow::Error;

    fn try_from(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let fixed_sized_bytes = <[u8; SIZE]>::try_from(bytes.as_slice()).map_err(|_| {
            anyhow!(
                "invalid bytes sequence length; expected {} bytes, got {} bytes",
                SIZE,
                bytes.len(),
            )
        })?;

        Ok(Hash(pallas::Hash::new(fixed_sized_bytes)))
    }
}

impl<const SIZE: usize> From<[u8; SIZE]> for Hash<SIZE> {
    fn from(hash: [u8; SIZE]) -> Self {
        Self::from(pallas::Hash::from(hash))
    }
}

impl<const SIZE: usize> From<pallas::Hash<SIZE>> for Hash<SIZE> {
    fn from(hash: pallas::Hash<SIZE>) -> Self {
        Self(hash)
    }
}

impl<const SIZE: usize> From<&pallas::Hash<SIZE>> for Hash<SIZE> {
    fn from(hash: &pallas::Hash<SIZE>) -> Self {
        Self(*hash)
    }
}

// ------------------------------------------------------------- Converting (to)

impl<const SIZE: usize> From<Hash<SIZE>> for pallas::Hash<SIZE> {
    fn from(hash: Hash<SIZE>) -> Self {
        hash.0
    }
}

impl<const SIZE: usize> From<&Hash<SIZE>> for pallas::Hash<SIZE> {
    fn from(hash: &Hash<SIZE>) -> Self {
        hash.0
    }
}

impl<const SIZE: usize> From<Hash<SIZE>> for [u8; SIZE] {
    fn from(hash: Hash<SIZE>) -> Self {
        *hash.0
    }
}

impl<const SIZE: usize> AsRef<[u8]> for Hash<SIZE> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Hash28(#[n(0)] Hash<28>);

#[cfg(feature = "wasm")]
impl From<Hash<28>> for Hash28 {
    fn from(hash28: Hash<28>) -> Self {
        Self(hash28)
    }
}

#[cfg(feature = "wasm")]
impl Deref for Hash28 {
    type Target = Hash<28>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Hash28 {
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toString"))]
    pub fn _wasm_to_string(&self) -> String {
        self.0.to_string()
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Hash32(#[n(0)] Hash<32>);

#[cfg(feature = "wasm")]
impl From<Hash<32>> for Hash32 {
    fn from(hash32: Hash<32>) -> Self {
        Self(hash32)
    }
}

#[cfg(feature = "wasm")]
impl Deref for Hash32 {
    type Target = Hash<32>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Hash32 {
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toString"))]
    pub fn _wasm_to_string(&self) -> String {
        self.0.to_string()
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::Hash;
    use proptest::prelude::*;

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        pub fn hash28() -> impl Strategy<Value = Hash<28>> {
            any::<[u8; 28]>().prop_map(Hash::from)
        }

        pub fn hash32() -> impl Strategy<Value = Hash<32>> {
            any::<[u8; 32]>().prop_map(Hash::from)
        }
    }
}
