use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::codec;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    base_url: String,
    #[n(1)]
    codec: codec::Kind,
}

impl Config {
    pub fn new(base_url: impl Into<String>, codec: codec::Kind) -> Self {
        Self {
            base_url: base_url.into(),
            codec,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn codec(&self) -> codec::Kind {
        self.codec
    }
}
