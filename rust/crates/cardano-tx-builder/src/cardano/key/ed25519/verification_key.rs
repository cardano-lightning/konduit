// Originally based on pallas:::crypto

use cryptoxide::{ed25519::verify, hashing::blake2b_224};

use super::{Signature, SigningKey};

use crate::Hash;

/// Ed25519 Verificaion Key.
/// EXTENDED KEYS ARE NOT SUPPORTED
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[repr(transparent)]
pub struct VerificationKey([u8; 32]);

impl VerificationKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn hash(&self) -> Hash<28> {
        Hash::from(blake2b_224(&self.0))
    }

    pub fn verify<T>(&self, message: T, signature: &Signature) -> bool
    where
        T: AsRef<[u8]>,
    {
        verify(message.as_ref(), &self.0, &signature.clone().bytes())
    }
}

impl From<SigningKey> for VerificationKey {
    fn from(value: SigningKey) -> Self {
        value.verification_key()
    }
}

impl From<[u8; 32]> for VerificationKey {
    fn from(value: [u8; 32]) -> Self {
        Self::new(value)
    }
}

impl From<VerificationKey> for [u8; 32] {
    fn from(value: VerificationKey) -> Self {
        value.bytes()
    }
}
