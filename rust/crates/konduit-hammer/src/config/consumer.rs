use std::collections::HashMap;

use cardano_sdk::{LeakableSigningKey, SigningKey};
use konduit_data::Tag;
use serde::{Deserialize, Serialize};

use crate::config::secret::SecretKey;

/// We "define" a consumer here as having a single key.
/// We restrict to the case where a consumer has 1-1 channel tag
/// (which _ought_ to be the case).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    key: super::SecretKey,
    /// Adapator name and tag!
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    channel: HashMap<Tag, String>,
}

impl super::Secret for Config {
    fn inject(&mut self, prefix: &str) {
        self.key.inject(&format!("{}_KEY", prefix));
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        self.key.extract(&format!("{}_KEY", prefix), env_list);
    }
}

impl Config {
    pub fn new(key: SecretKey) -> Self {
        Self {
            key,
            channel: HashMap::new(),
        }
    }

    pub fn key(&self) -> SigningKey {
        self.key.inner.as_ref().unwrap().clone().into_signing_key()
    }

    pub fn channel(&self) -> &HashMap<Tag, String> {
        &self.channel
    }

    pub fn generate() -> Self {
        Self::new(SecretKey::generate())
    }

    pub fn examples() -> HashMap<String, Self> {
        HashMap::from([
            ("charlie".to_string(), Self::generate()),
            ("carol".to_string(), Self::generate()),
        ])
    }

    pub fn add_channel(&mut self, tag: Tag, adaptor: String) {
        self.channel.insert(tag, adaptor);
    }
}
