//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use anyhow::anyhow;
use std::fmt;

/// A wrapper around the _blake2b-224_ hash digest of a key or script.
///
/// It behaves like a enum with two variants, although the constructors are kept private to avoid
/// leaking implementation internals. One can manipulate either of the two variants by using the
/// higher-level API:
///
/// - [`Self::as_key`]
/// - [`Self::as_script`]
///
/// If something more fine-grained is needed where either are needed, one may simply use:
///
/// - [`Self::select`]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Credential(#[n(0)] pallas::StakeCredential);

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(
            self.select(
                |hash| format!("Key({hash})"),
                |hash| format!("Script({hash})"),
            )
            .as_str(),
        )
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
    /// Construct a credential from a key.
    ///
    /// See also [`key_credential!`](crate::key_credential).
    pub fn from_key(hash: Hash<28>) -> Self {
        Self::from(pallas::StakeCredential::AddrKeyhash(pallas::Hash::from(
            hash,
        )))
    }

    /// Construct a credential from a script.
    ///
    /// See also [`script_credential!`](crate::script_credential).
    pub fn from_script(hash: Hash<28>) -> Self {
        Self::from(pallas::StakeCredential::ScriptHash(pallas::Hash::from(
            hash,
        )))
    }
}

// ------------------------------------------------------------------ Inspecting

impl Credential {
    /// Run a computation (possibly the same) for either of the two variants.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::{script_credential};
    /// assert_eq!(
    ///   script_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777")
    ///     .select(
    ///         |_| "is_key".to_string(),
    ///         |_| "is_script".to_string(),
    ///     ),
    ///   "is_script"
    /// );
    /// ```
    ///
    /// ```rust
    /// # use cardano_tx_builder::{key_credential};
    /// assert_eq!(
    ///   key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777")
    ///     .select(
    ///         |hash| format!("Key({hash})"),
    ///         |hash| format!("Script({hash})"),
    ///     ),
    ///   "Key(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777)"
    /// )
    /// ```
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

    /// Continues with the inner hash, provided that the credential is that of a key.
    pub fn as_key(&self) -> Option<Hash<28>> {
        self.select(Some, |_| None)
    }

    /// Continues with the inner hash, provided that the credential is that of a script.
    pub fn as_script(&self) -> Option<Hash<28>> {
        self.select(|_| None, Some)
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

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::{Credential, any, key_credential, pallas, script_credential};
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_key() {
        assert_eq!(
            key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777").to_string(),
            "Key(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777)",
        );
    }

    #[test]
    fn display_script() {
        assert_eq!(
            script_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777")
                .to_string(),
            "Script(bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777)",
        );
    }

    // -------------------------------------------------------------- Properties

    proptest! {
        #[test]
        fn pallas_roundtrip(credential in any::credential()) {
            let pallas_credential = pallas::StakeCredential::from(credential.clone());
            let credential_back = Credential::from(pallas_credential);
            prop_assert_eq!(credential, credential_back);
        }
    }

    proptest! {
        #[test]
        fn from_key_roundtrip(hash in any::hash28()) {
            prop_assert!(
                Credential::from_key(hash)
                    .as_key()
                    .is_some_and(|inner_hash| inner_hash == hash)
            )
        }
    }

    proptest! {
        #[test]
        fn from_script_roundtrip(hash in any::hash28()) {
            prop_assert!(
                Credential::from_script(hash)
                    .as_script()
                    .is_some_and(|inner_hash| inner_hash == hash)
            )
        }
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        pub fn credential() -> impl Strategy<Value = Credential> {
            prop_oneof![
                any::hash28().prop_map(Credential::from_key),
                any::hash28().prop_map(Credential::from_script),
            ]
        }
    }
}
