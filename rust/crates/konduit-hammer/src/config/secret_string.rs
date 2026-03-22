use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::env;
/// A wrapper for sensitive strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecretString(pub String);

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString(s))
    }
}

impl super::Secret for SecretString {
    fn inject(&mut self, _prefix: &str) {
        // Any string starting with $ is treated as an env var key
        if self.0.starts_with('$') && self.0.len() > 1 {
            let key = &self.0[1..];
            if let Ok(val) = env::var(key) {
                self.0 = val;
            }
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        // If the value is a real value (doesn't start with $) and isn't empty, extract it.
        if !self.0.starts_with('$') && !self.0.is_empty() {
            env_list.push(format!("{}={}", prefix, self.0));
            // Store as a placeholder trigger for next load
            self.0 = format!("${}", prefix);
        }
    }
}


