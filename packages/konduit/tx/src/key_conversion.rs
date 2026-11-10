use cardano_sdk::{Signature as CardanoSignature, SigningKey as CardanoSigningKey};
use konduit_core::{Signature, SigningKey, VerifyingKey};

/// Convert a Cardano signing key into the protocol key type, consuming the
/// Cardano wrapper so the secret is not left in an additional allocation.
pub fn to_signing_key(key: CardanoSigningKey) -> SigningKey {
    // SAFETY: This deliberately transfers the secret bytes into the protocol
    // key representation. `SigningKey` owns the resulting bytes; callers must
    // treat that value as secret and must not expose it through logs or output.
    unsafe { CardanoSigningKey::leak(key) }.into()
}

pub fn from_signing_key(key: SigningKey) -> CardanoSigningKey {
    <[u8; 32]>::from(key).into()
}

pub fn to_verifying_key(key: cardano_sdk::VerificationKey) -> VerifyingKey {
    <[u8; 32]>::from(key).into()
}

pub fn from_verifying_key(key: VerifyingKey) -> cardano_sdk::VerificationKey {
    <[u8; 32]>::from(key).into()
}

pub fn to_signature(signature: CardanoSignature) -> Signature {
    <[u8; 64]>::from(signature).into()
}

pub fn from_signature(signature: Signature) -> CardanoSignature {
    <[u8; 64]>::from(signature).into()
}
