use std::{fmt::Display, str::FromStr};

use cardano_tx_builder::{Address, Credential, Hash, NetworkId, VerificationKey, address::kind};
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// No-holds-barred signing key.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SigningKey(#[serde_as(as = "serde_with::hex::Hex")] [u8; 32]);

impl AsRef<[u8]> for SigningKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for SigningKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?;
        let array: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
            anyhow::anyhow!("Wrong length: expected 32 bytes, got {}", v.len())
        })?;
        Ok(SigningKey(array))
    }
}

impl From<[u8; 32]> for SigningKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<Vec<u8>> for SigningKey {
    type Error = &'static str;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err("Invalid key length: expected 32 bytes");
        }

        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&value);
        Ok(Self(bytes))
    }
}

impl From<SigningKey> for cardano_tx_builder::SigningKey {
    fn from(key: SigningKey) -> Self {
        Self::from(key.0)
    }
}

impl SigningKey {
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.try_fill_bytes(&mut key).unwrap();
        Self(key)
    }

    pub fn to_address(&self, network_id: &NetworkId) -> Address<kind::Shelley> {
        Address::new(network_id.clone(), Credential::from_key(self.to_vkh()))
    }

    pub fn to_verification_key(&self) -> VerificationKey {
        VerificationKey::from(&<cardano_tx_builder::SigningKey>::from(self.clone()))
    }

    pub fn to_vkh(&self) -> Hash<28> {
        Hash::<28>::new(&self.to_verification_key())
    }
}
