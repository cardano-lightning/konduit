use std::collections::HashMap;

use http_client_native::HttpClient;
use serde::{Deserialize, Serialize};

use crate::{Cardano, config::secret::SecretString};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Config {
    Client(Client),
    Blockfrost(Blockfrost),
}

impl Config {
    pub fn examples() -> HashMap<String, Self> {
        HashMap::from([
            ("client".to_string(), Self::Client(Client::example())),
            (
                "blockfrost".to_string(),
                Self::Blockfrost(Blockfrost::example()),
            ),
        ])
    }

    pub async fn build(self) -> anyhow::Result<Cardano> {
        let client = match self {
            Config::Client(config) => Cardano::Client(config.build().await?),
            Config::Blockfrost(config) => Cardano::Blockfrost(config.build()),
        };
        Ok(client)
    }
}

impl super::Secret for Config {
    fn inject(&mut self, prefix: &str) {
        match self {
            Config::Client(config) => config.inject(prefix),
            Config::Blockfrost(config) => config.inject(prefix),
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        match self {
            Config::Client(config) => config.extract(prefix, env_list),
            Config::Blockfrost(config) => config.extract(prefix, env_list),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub url: String,
}

impl Client {
    pub fn example() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
        }
    }

    pub async fn build(self) -> anyhow::Result<cardano_connector_client::Connector<HttpClient>> {
        let http = HttpClient::new(&self.url);
        let res = cardano_connector_client::Connector::new(http).await?;
        Ok(res)
    }
}

impl super::Secret for Client {
    fn inject(&mut self, _prefix: &str) {}

    fn extract(&mut self, _prefix: &str, _env_list: &mut Vec<String>) {}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Blockfrost {
    pub project_id: SecretString,
}

impl Blockfrost {
    pub fn example() -> Self {
        Self {
            project_id: SecretString::new("mainnetXXXXXXXXXXXXXXXXXXXX".to_string()),
        }
    }

    pub fn build(self) -> cardano_connector_direct::Blockfrost {
        cardano_connector_direct::Blockfrost::new(
            self.project_id
                .inner
                .expect("Project id expected but none given"),
        )
    }
}

impl super::Secret for Blockfrost {
    fn inject(&mut self, prefix: &str) {
        self.project_id.inject(&format!("{}_PROJECT_ID", prefix));
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        self.project_id
            .extract(&format!("{}_PROJECT_ID", prefix), env_list);
    }
}
