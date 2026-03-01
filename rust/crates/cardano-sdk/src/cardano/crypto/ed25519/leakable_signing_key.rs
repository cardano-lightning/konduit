//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::SigningKey;
use std::{ops::Deref, str::FromStr};

/// An ed25519 signing key which leaks through its serde::Serialised instance. Used in
/// command-lines and interfaces.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct LeakableSigningKey(
    #[cfg_attr(
        feature = "serde",
        serde(
            serialize_with = "LeakableSigningKey::unsafe_serialize",
            deserialize_with = "LeakableSigningKey::deserialize"
        )
    )]
    SigningKey,
);

impl Deref for LeakableSigningKey {
    type Target = SigningKey;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl LeakableSigningKey {
    pub fn into_signing_key(self) -> SigningKey {
        SigningKey::from(self)
    }
}

impl From<LeakableSigningKey> for SigningKey {
    fn from(lsk: LeakableSigningKey) -> Self {
        lsk.0
    }
}

impl From<SigningKey> for LeakableSigningKey {
    fn from(sk: SigningKey) -> Self {
        LeakableSigningKey(sk)
    }
}

impl FromStr for LeakableSigningKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(Self::from(SigningKey::from_str(s)?))
    }
}

impl LeakableSigningKey {
    #[cfg(feature = "serde")]
    pub fn unsafe_serialize<S: serde::ser::Serializer>(
        sk: &SigningKey,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&unsafe { hex::encode(SigningKey::leak(sk.clone())) })
    }

    #[cfg(feature = "serde")]
    pub fn deserialize<'de, D: serde::de::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SigningKey, D::Error> {
        let text: &str = serde::Deserialize::deserialize(deserializer)?;
        SigningKey::from_str(text).map_err(serde::de::Error::custom)
    }
}
