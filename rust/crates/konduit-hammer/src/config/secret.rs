use bln_client::lnd::Macaroon;
use cardano_sdk::{LeakableSigningKey, SigningKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::env;
use std::str::FromStr;

/// Bridge for raw strings, binary types, and cryptographic keys.
pub trait SecretCodec: Sized {
    fn decode(s: &str) -> anyhow::Result<Self>;
    fn encode(&self) -> String;
}

/// Unified wrapper for environment substitution and sensitive data management.
#[derive(Debug, Clone, Default)]
pub struct SecretBox<T: SecretCodec> {
    pub inner: Option<T>,
    pub raw: String,
}

impl<T: SecretCodec> SecretBox<T> {
    pub fn new(val: T) -> Self {
        Self {
            raw: val.encode(),
            inner: Some(val),
        }
    }
}

impl<T: SecretCodec> Serialize for SecretBox<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.raw)
    }
}

impl<'de, T: SecretCodec> Deserialize<'de> for SecretBox<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(SecretBox {
            inner: None,
            raw: String::deserialize(d)?,
        })
    }
}

pub trait Secret {
    fn inject(&mut self, prefix: &str);
    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>);
}

impl<T: SecretCodec> Secret for SecretBox<T> {
    fn inject(&mut self, _prefix: &str) {
        if self.raw.starts_with('$') && self.raw.len() > 1 {
            if let Ok(val) = env::var(&self.raw[1..]) {
                self.raw = val;
            }
        }
        if !self.raw.starts_with('$') && !self.raw.is_empty() {
            self.inner = T::decode(&self.raw).ok();
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        if let Some(ref val) = self.inner {
            if !self.raw.starts_with('$') {
                env_list.push(format!("{}={}", prefix, val.encode()));
                self.raw = format!("${}", prefix);
            }
        }
    }
}

// --- Implementations ---

impl SecretCodec for String {
    fn decode(s: &str) -> anyhow::Result<Self> {
        Ok(s.to_string())
    }
    fn encode(&self) -> String {
        self.clone()
    }
}

impl SecretCodec for LeakableSigningKey {
    fn decode(s: &str) -> anyhow::Result<Self> {
        Self::from_str(s)
    }
    fn encode(&self) -> String {
        unsafe { hex::encode(SigningKey::leak(self.clone().into_signing_key())) }
    }
}

impl SecretCodec for Vec<u8> {
    fn decode(s: &str) -> anyhow::Result<Self> {
        hex::decode(s).map_err(anyhow::Error::msg)
    }
    fn encode(&self) -> String {
        hex::encode(&self)
    }
}

impl SecretCodec for Macaroon {
    fn decode(s: &str) -> anyhow::Result<Self> {
        hex::decode(s).map(Self::from).map_err(anyhow::Error::msg)
    }
    fn encode(&self) -> String {
        hex::encode(self.as_ref())
    }
}

pub type SecretString = SecretBox<String>;
pub type SecretBytes = SecretBox<Vec<u8>>;
pub type SecretMacaroon = SecretBox<Macaroon>;

pub type SecretKey = SecretBox<LeakableSigningKey>;

impl SecretKey {
    pub fn generate() -> Self {
        SecretBox::new(LeakableSigningKey::from(SigningKey::new()))
    }
}
