use cardano_sdk::{Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_with::{hex::Hex, serde_as};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionClaimRequest {
    #[serde_as(as = "Hex")]
    pub wallet_verification_key_hex: [u8; 32],
    #[serde_as(as = "Hex")]
    pub adaptor_verification_key_hex: [u8; 32],
    #[serde(deserialize_with = "deserialize_generation")]
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
            "{}\n{}\n{}\n{}\n{}",
            hex::encode(self.adaptor_verification_key_hex),
            self.generation,
            hex::encode(self.backup_hash_hex),
            hex::encode(self.device_public_key_hex),
            self.timestamp
        )
    }

    pub fn signed(
        signing_key: &SigningKey,
        adaptor_verification_key: [u8; 32],
        generation: u64,
        backup_hash: [u8; 32],
        device_public_key: [u8; 32],
        timestamp: u64,
    ) -> Self {
        assert!(generation > 0, "session generation must be at least 1");
        let mut claim = Self {
            wallet_verification_key_hex: signing_key.to_verification_key().into(),
            adaptor_verification_key_hex: adaptor_verification_key,
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
        self.generation > 0
            && VerificationKey::from(self.wallet_verification_key_hex).verify(
                self.message().as_bytes(),
                &Signature::from(self.signature_hex),
            )
    }
}

fn deserialize_generation<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let generation = u64::deserialize(deserializer)?;
    (generation > 0)
        .then_some(generation)
        .ok_or_else(|| de::Error::custom("session generation must be at least 1"))
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
        SessionClaimRequest::signed(
            &SigningKey::from([7; 32]),
            [9; 32],
            3,
            [0xab; 32],
            [0xcd; 32],
            42,
        )
    }

    #[test]
    fn message_has_canonical_format() {
        assert_eq!(
            claim().message(),
            format!(
                "{}\n3\n{}\n{}\n42",
                "09".repeat(32),
                "ab".repeat(32),
                "cd".repeat(32)
            )
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
        changed.adaptor_verification_key_hex[0] ^= 1;
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

    #[test]
    fn zero_generation_is_rejected() {
        let mut invalid = serde_json::to_value(claim()).unwrap();
        invalid["generation"] = json!(0);
        assert!(serde_json::from_value::<SessionClaimRequest>(invalid).is_err());
    }

    #[test]
    #[should_panic(expected = "session generation must be at least 1")]
    fn signed_rejects_zero_generation() {
        SessionClaimRequest::signed(&SigningKey::from([7; 32]), [9; 32], 0, [0; 32], [0; 32], 0);
    }
}
