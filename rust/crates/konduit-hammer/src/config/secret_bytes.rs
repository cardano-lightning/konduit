use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::env;
use std::str::FromStr;
use std::fmt::Display;

/// A generic wrapper for sensitive byte arrays (Vec<u8>).
/// T must implement FromStr (for parsing hex/b64) and Display (for serializing back).
#[derive(Debug, Clone, Default)]
pub struct SecretBytes<T> {
    pub data: Option<T>,
    pub raw: String,
}

impl<T> SecretBytes<T> 
where 
    T: FromStr + Display 
{
    pub fn new(data: T) -> Self {
        Self {
            raw: data.to_string(),
            data: Some(data),
        }
    }
}

// --- Serde Implementation (Identical to your SecretKey) ---

impl<T> Serialize for SecretBytes<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de, T> Deserialize<'de> for SecretBytes<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        Ok(SecretBytes { data: None, raw: s })
    }
}

// --- The Secret Trait Logic (Reusable) ---

impl<T> super::Secret for SecretBytes<T> 
where 
    T: FromStr + Display 
{
    fn inject(&mut self, _prefix: &str) {
        if self.raw.starts_with('$') && self.raw.len() > 1 {
            let var_name = &self.raw[1..];
            if let Ok(val) = env::var(var_name) {
                self.raw = val;
            }
        }

        if !self.raw.starts_with('$') && !self.raw.is_empty() {
            // Attempt to parse the raw string into the specific byte-wrapper T
            if let Ok(parsed) = T::from_str(&self.raw) {
                self.data = Some(parsed);
            }
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        if let Some(ref data_obj) = self.data {
            if !self.raw.starts_with('$') {
                let val = data_obj.to_string();
                env_list.push(format!("{}={}", prefix, val));
                self.raw = format!("${}", prefix);
            }
        }
    }
}
