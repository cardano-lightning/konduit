use cardano_sdk::{NetworkId, address::kind::Shelley};
use konduit_data::SigningKey;
use serde::{Deserialize, Serialize};

use crate::hash32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    key: SigningKey,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key: hash32("wallet".as_bytes()).into(),
        }
    }
}

impl Config {
    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }

    pub fn verification_key(&self) -> cardano_sdk::VerificationKey {
        self.cardano_signing_key().to_verification_key()
    }

    pub fn credential(&self) -> cardano_sdk::Credential {
        cardano_sdk::Credential::from_key(cardano_sdk::Hash::<28>::new(self.verification_key()))
    }

    pub fn address(&self, network_id: NetworkId) -> cardano_sdk::Address<Shelley> {
        cardano_sdk::Address::new(network_id, self.credential())
    }
}
