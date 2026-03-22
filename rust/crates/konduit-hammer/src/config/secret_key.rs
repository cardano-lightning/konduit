use cardano_sdk::{LeakableSigningKey, SigningKey};
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::env;

/// A wrapper for sensitive Cardano signing keys.
/// It handles hex serialization and environment variable substitution.
#[derive(Debug, Clone)]
pub struct SecretKey {
    /// The decoded cryptographic key, populated after injection.
    pub key: Option<LeakableSigningKey>,
    /// The raw string representation (either hex or a $VARIABLE).
    pub raw: String,
}

impl Default for SecretKey {
    fn default() -> Self {
        Self {
            key: None,
            raw: String::new(),
        }
    }
}

impl SecretKey {
    pub fn new() -> Self {
        let key = Some(LeakableSigningKey::from(SigningKey::new()));
        Self { key, raw: String::new(), }
    }
}

impl Serialize for SecretKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // We always serialize the string representation (either the hex or the $VAR)
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecretKey {
            key: None,
            raw: s,
        })
    }
}

impl super::Secret for SecretKey {
    fn inject(&mut self, _prefix: &str) {
        // 1. Handle Environment Substitution if string starts with $
        if self.raw.starts_with('$') && self.raw.len() > 1 {
            let key_name = &self.raw[1..];
            if let Ok(val) = env::var(key_name) {
                self.raw = val;
            }
        }

        // 2. Attempt to parse the string as a hex key
        if !self.raw.starts_with('$') && !self.raw.is_empty() {
            if let Ok(key) = self.raw.parse::<LeakableSigningKey>() {
                self.key = Some(key);
            }
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        // If we have a real key but no placeholder string, generate the hex and the placeholder
        if let Some(ref key_obj) = self.key {
            if !self.raw.starts_with('$') {
                let hex_val = serde_json::to_string(key_obj).unwrap();
                env_list.push(format!("{}={}", prefix, hex_val));
                self.raw = format!("${}", prefix);
            }
        } else if !self.raw.starts_with('$') && !self.raw.is_empty() {
            // If we only have a raw string (maybe it failed to parse as key), extract it anyway
            env_list.push(format!("{}={}", prefix, self.raw));
            self.raw = format!("${}", prefix);
        }
    }
}
