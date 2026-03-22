use std::collections::HashMap;

use http_client_native::HttpClient;
use serde::{Deserialize, Serialize};

use crate::Adaptor;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    url: String,
}

impl super::Secret for Config {
    fn inject(&mut self, _prefix: &str) {}

    fn extract(&mut self, _prefix: &str, _env_list: &mut Vec<String>) {}
}

impl Config {
    pub fn examples() -> HashMap<String, Self> {
        HashMap::from([(
            "local".to_string(),
            Self {
                url: "http://localhost:5663".to_string(),
            },
        )])
    }

    pub fn build(self) -> Adaptor<HttpClient> {
        Adaptor::new(HttpClient::new(self.url.as_str()))
    }
}
