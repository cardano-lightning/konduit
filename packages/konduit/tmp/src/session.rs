use cardano_sdk::{Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionClaimRequest {
    #[serde_as(as = "Hex")]
    pub wallet_verification_key_hex: [u8; 32],
    pub generation: u64,
    #[serde_as(as = "Hex")]
    pub backup_hash_hex: [u8; 32],
    #[serde_as(as = "Hex")]
    pub device_public_key_hex: [u8; 32],
    pub timestamp: u64,
    #[serde_as(as = "Hex")]
    pub signature_hex: [u8; 64],
}

impl SessionClaimRequest {
    pub fn message(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}",
            self.generation,
            hex::encode(self.backup_hash_hex),
            hex::encode(self.device_public_key_hex),
            self.timestamp
        )
    }

    pub fn signed(
        signing_key: &SigningKey,
        generation: u64,
        backup_hash: [u8; 32],
        device_public_key: [u8; 32],
        timestamp: u64,
    ) -> Self {
        let mut claim = Self {
            wallet_verification_key_hex: signing_key.to_verification_key().into(),
            generation,
            backup_hash_hex: backup_hash,
            device_public_key_hex: device_public_key,
            timestamp,
            signature_hex: [0; 64],
        };
        claim.signature_hex = signing_key.sign(claim.message().as_bytes()).into();
        claim
    }

    pub fn verify(&self) -> bool {
        VerificationKey::from(self.wallet_verification_key_hex).verify(
            self.message().as_bytes(),
            &Signature::from(self.signature_hex),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionClaimResponse {
    pub lease: String,
    pub expires_at_epoch_millis: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claim() -> SessionClaimRequest {
        SessionClaimRequest::signed(&SigningKey::from([7; 32]), 3, [0xab; 32], [0xcd; 32], 42)
    }

    #[test]
    fn message_has_canonical_format() {
        assert_eq!(
            claim().message(),
            format!("3\n{}\n{}\n42", "ab".repeat(32), "cd".repeat(32))
        );
    }

    #[test]
    fn signed_claim_verifies_and_mutations_fail() {
        let claim = claim();
        assert!(claim.verify());

        let mut changed = claim.clone();
        changed.generation += 1;
        assert!(!changed.verify());
        changed = claim.clone();
        changed.backup_hash_hex[0] ^= 1;
        assert!(!changed.verify());
        changed = claim.clone();
        changed.device_public_key_hex[0] ^= 1;
        assert!(!changed.verify());
        changed = claim.clone();
        changed.timestamp += 1;
        assert!(!changed.verify());
        changed = claim;
        changed.signature_hex[0] ^= 1;
        assert!(!changed.verify());
    }

    #[test]
    fn malformed_or_wrong_length_hex_is_rejected() {
        let valid = serde_json::to_value(claim()).unwrap();
        for (field, value) in [
            ("walletVerificationKeyHex", "00"),
            ("backupHashHex", "gg"),
            ("devicePublicKeyHex", "00"),
            ("signatureHex", "00"),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = json!(value);
            assert!(serde_json::from_value::<SessionClaimRequest>(invalid).is_err());
        }
    }
}
