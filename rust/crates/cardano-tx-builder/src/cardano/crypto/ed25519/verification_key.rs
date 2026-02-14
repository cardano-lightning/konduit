//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    Address, Credential, NetworkId, PlutusData, Signature, address::kind, pallas::ed25519,
};
use std::{cmp, fmt, str::FromStr};

/// A ed25519 verification key (non-extended).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct VerificationKey(pub ed25519::PublicKey);

impl fmt::Display for VerificationKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl PartialOrd for VerificationKey {
    fn partial_cmp(&self, rhs: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for VerificationKey {
    fn cmp(&self, rhs: &Self) -> cmp::Ordering {
        let lhs = self.as_ref();
        let rhs = rhs.as_ref();
        lhs.cmp(rhs)
    }
}

// ------------------------------------------------------------------ Inspecting

impl VerificationKey {
    pub const SIZE: usize = ed25519::PublicKey::SIZE;

    /// Verify a [`Signature`] against the given [`VerificationKey`]. Returns `true` when the
    /// signature is valid.
    pub fn verify<T>(&self, message: T, signature: &Signature) -> bool
    where
        T: AsRef<[u8]>,
    {
        self.0
            .verify(message, <&ed25519::Signature>::from(signature))
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<[u8; 32]> for VerificationKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(ed25519::PublicKey::from(bytes))
    }
}

impl TryFrom<Vec<u8>> for VerificationKey {
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self::from(<[u8; 32]>::try_from(value)?))
    }
}

impl From<ed25519::PublicKey> for VerificationKey {
    fn from(key: ed25519::PublicKey) -> Self {
        Self(key)
    }
}

impl FromStr for VerificationKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(VerificationKey(ed25519::PublicKey::from_str(s)?))
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for VerificationKey {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let array =
            <&'_ [u8; 32]>::try_from(data).map_err(|e| e.context("invalid verification key"))?;
        Ok(Self(ed25519::PublicKey::from(*array)))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for VerificationKey {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(&data)
    }
}

// ------------------------------------------------------------- Converting (to)

impl VerificationKey {
    pub fn to_address(&self, network_id: NetworkId) -> Address<kind::Shelley> {
        Address::new(network_id, self.to_credential())
    }

    pub fn to_credential(&self) -> Credential {
        Credential::from(self)
    }

    pub fn as_plutus_data<'a>(&'a self) -> PlutusData<'a> {
        PlutusData::from(self)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.as_ref()
    }

    pub fn into_bytes(self) -> [u8; 32] {
        <[u8; 32]>::from(self)
    }
}

impl AsRef<[u8]> for VerificationKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<VerificationKey> for [u8; 32] {
    fn from(key: VerificationKey) -> Self {
        key.0.into()
    }
}

impl From<VerificationKey> for ed25519::PublicKey {
    fn from(key: VerificationKey) -> Self {
        key.0
    }
}

impl<'a> From<&'a VerificationKey> for &'a ed25519::PublicKey {
    fn from(key: &'a VerificationKey) -> Self {
        &key.0
    }
}

impl<'a> From<&'a VerificationKey> for PlutusData<'a> {
    fn from(key: &'a VerificationKey) -> Self {
        Self::bytes(key.0)
    }
}
