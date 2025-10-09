//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use anyhow::anyhow;
use std::fmt;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Hash<const SIZE: usize>(#[n(0)] pallas::Hash<SIZE>);

impl<const SIZE: usize> fmt::Display for Hash<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
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
