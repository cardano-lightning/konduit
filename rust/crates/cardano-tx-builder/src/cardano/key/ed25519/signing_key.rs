// Originally based on pallas:::crypto

use cryptoxide::ed25519::{keypair, signature};

use super::{Signature, VerificationKey};

/// Ed25519 Signing Key.
/// EXTENDED KEYS ARE NOT SUPPORTED
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[repr(transparent)]
pub struct SigningKey([u8; 32]);

impl SigningKey {
    /// FIXME:: no scrubbing.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn leak(self) -> [u8; 32] {
        self.0
    }

    pub fn verification_key(&self) -> VerificationKey {
        let (mut _both, public) = keypair(&self.0);
        VerificationKey::from(public)
    }

    pub fn sign<T>(&self, msg: T) -> Signature
    where
        T: AsRef<[u8]>,
    {
        let (mut both, _public) = keypair(&self.0);
        let signature = signature(msg.as_ref(), &mut both);
        Signature::from(signature)
    }
}

impl From<[u8; 32]> for SigningKey {
    fn from(value: [u8; 32]) -> Self {
        Self::new(value)
    }
}

impl From<SigningKey> for [u8; 32] {
    fn from(value: SigningKey) -> Self {
        value.leak()
    }
}
