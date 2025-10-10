//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use anyhow::anyhow;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Credential(#[n(0)] pallas::StakeCredential);

// ------------------------------------------------------------------ Inspecting

impl Credential {
    pub fn select<T>(
        &self,
        mut when_key: impl FnMut(Hash<28>) -> T,
        mut when_script: impl FnMut(Hash<28>) -> T,
    ) -> T {
        match &self.0 {
            pallas::StakeCredential::AddrKeyhash(hash) => when_key(Hash::from(hash)),
            pallas::StakeCredential::ScriptHash(hash) => when_script(Hash::from(hash)),
        }
    }

    pub fn as_key(&self) -> Option<Hash<28>> {
        self.select(Some, |_| None)
    }

    pub fn as_script(&self) -> Option<Hash<28>> {
        self.select(|_| None, Some)
    }
}

// -------------------------------------------------------------------- Building

impl Default for Credential {
    fn default() -> Self {
        Self::from_key(Hash::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]))
    }
}

impl Credential {
    pub fn from_key(hash: Hash<28>) -> Self {
        Self::from(pallas::StakeCredential::AddrKeyhash(pallas::Hash::from(
            hash,
        )))
    }

    pub fn from_script(hash: Hash<28>) -> Self {
        Self::from(pallas::StakeCredential::ScriptHash(pallas::Hash::from(
            hash,
        )))
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::StakeCredential> for Credential {
    fn from(credential: pallas::StakeCredential) -> Self {
        Self(credential)
    }
}

impl From<&pallas::ShelleyPaymentPart> for Credential {
    fn from(payment_part: &pallas::ShelleyPaymentPart) -> Self {
        match payment_part {
            pallas_addresses::ShelleyPaymentPart::Key(hash) => {
                Self(pallas::StakeCredential::AddrKeyhash(*hash))
            }
            pallas_addresses::ShelleyPaymentPart::Script(hash) => {
                Self(pallas::StakeCredential::ScriptHash(*hash))
            }
        }
    }
}

impl TryFrom<&pallas::ShelleyDelegationPart> for Credential {
    type Error = anyhow::Error;

    fn try_from(delegation_part: &pallas::ShelleyDelegationPart) -> anyhow::Result<Self> {
        match delegation_part {
            pallas_addresses::ShelleyDelegationPart::Key(hash) => {
                Ok(Self(pallas::StakeCredential::AddrKeyhash(*hash)))
            }
            pallas_addresses::ShelleyDelegationPart::Script(hash) => {
                Ok(Self(pallas::StakeCredential::ScriptHash(*hash)))
            }
            pallas_addresses::ShelleyDelegationPart::Pointer(..) => {
                Err(anyhow!("unsupported pointer address")
                    .context(format!("delegation part={:?}", delegation_part)))
            }
            pallas_addresses::ShelleyDelegationPart::Null => Err(anyhow!("no delegation part")),
        }
    }
}

// ------------------------------------------------------------- Converting (to)

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut bytes = Vec::new();
        let _ = cbor::encode(self, &mut bytes);
        f.write_str(hex::encode(bytes.as_slice()).as_str())
    }
}

impl From<Credential> for pallas::StakeCredential {
    fn from(credential: Credential) -> Self {
        credential.0
    }
}

impl From<Credential> for pallas::ShelleyPaymentPart {
    fn from(credential: Credential) -> Self {
        match credential.0 {
            pallas::StakeCredential::AddrKeyhash(hash) => pallas::ShelleyPaymentPart::Key(hash),
            pallas::StakeCredential::ScriptHash(hash) => pallas::ShelleyPaymentPart::Script(hash),
        }
    }
}

impl From<Credential> for pallas::ShelleyDelegationPart {
    fn from(credential: Credential) -> Self {
        match credential.0 {
            pallas::StakeCredential::AddrKeyhash(hash) => pallas::ShelleyDelegationPart::Key(hash),
            pallas::StakeCredential::ScriptHash(hash) => {
                pallas::ShelleyDelegationPart::Script(hash)
            }
        }
    }
}
