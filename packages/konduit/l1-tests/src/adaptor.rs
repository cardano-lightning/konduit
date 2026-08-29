use konduit_data::{Duration, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::hash32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub key: SigningKey,
    pub close_period: Duration,
}

impl Config {
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    pub fn constants(&self) -> (VerifyingKey, Duration) {
        (self.key.verifying_key(), self.close_period)
    }

    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key: hash32("adaptor".as_bytes()).into(),
            close_period: Duration::from_secs(300),
        }
    }
}
