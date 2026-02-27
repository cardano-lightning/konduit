//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Signature, VerificationKey, pallas::ed25519};
use anyhow::anyhow;
use rand_core::RngCore;
use std::str::FromStr;

/// An ed25519 signing key (non-extended).
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct SigningKey(ed25519::SecretKey);

// ----------------------------------------------------------------------- Using

impl SigningKey {
    pub const SIZE: usize = ed25519::SecretKey::SIZE;

    /// Generate a new signing key using available system entropy.
    pub fn new() -> Self {
        let mut bytes = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut bytes);
        Self::from(bytes)
    }

    /// Convert the [`SecretKey`] into its compressed byte composition, and leak its bytes into
    /// memory. Only use for storing the key securely or for testing. Additional precautions are
    /// needed to ensure that the leaked bytes are properly de-allocated and cleared.
    ///
    /// # Safety
    ///
    /// This function is not safe because:
    ///
    /// * using it removes all the security measure we put in place
    ///   to protect your private key: opaque [`Debug`] impl, zeroisation on [`Drop`], ...
    /// * you will need to be careful not to leak the bytes
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano_sdk::SigningKey;
    /// let key: SigningKey = // ...
    /// # [0; SigningKey::SIZE].into() ;
    /// let _: [u8; SigningKey::SIZE] = unsafe { SigningKey::leak(key) };
    #[inline]
    pub unsafe fn leak(key: Self) -> [u8; 32] {
        unsafe { ed25519::SecretKey::leak_into_bytes(key.0) }
    }

    pub fn sign<T>(&self, msg: T) -> Signature
    where
        T: AsRef<[u8]>,
    {
        Signature::from(self.0.sign(msg))
    }
}

impl Default for SigningKey {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<[u8; 32]> for SigningKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(ed25519::SecretKey::from(bytes))
    }
}

impl TryFrom<Vec<u8>> for SigningKey {
    type Error = anyhow::Error;

    fn try_from(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let array = <[u8; 32]>::try_from(bytes).map_err(|_| anyhow!("invalid signing key"))?;
        Ok(Self::from(array))
    }
}

impl FromStr for SigningKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let bytes =
            hex::decode(s).map_err(|e| anyhow!(e).context("malformed base16 signing key"))?;

        let array = <[u8; 32]>::try_from(bytes).map_err(|_| anyhow!("invalid signing key"))?;

        Ok(Self::from(array))
    }
}

// ------------------------------------------------------------- Converting (to)

impl SigningKey {
    pub fn to_verification_key(&self) -> VerificationKey {
        VerificationKey::from(self)
    }
}

impl From<&SigningKey> for VerificationKey {
    fn from(key: &SigningKey) -> Self {
        VerificationKey::from(key.0.public_key())
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::{
        wasm::{self, Signature},
        wasm_proxy,
    };
    use std::str::FromStr;
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[derive(Debug, Clone)]
        #[doc = "An Ed25519 signing key (non-extended)."]
        SigningKey
    }

    #[wasm_bindgen]
    impl SigningKey {
        /// Construct a new signing key from a 64-digit hex-encoded text string. Throws if the
        /// string is malformed.
        #[wasm_bindgen(constructor)]
        pub fn _wasm_new(value: &str) -> wasm::Result<Self> {
            Ok(super::SigningKey::from_str(value)?.into())
        }

        /// Sign the given payload (unsafe if unknown /!\) with the key.
        #[wasm_bindgen(js_name = "sign")]
        pub fn _wasm_sign(&self, msg: &[u8]) -> Signature {
            self.sign(msg).into()
        }
    }
}
