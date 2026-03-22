use std::{collections::HashMap, time::Duration};

use bln_client::{MerchantApi, lnd::Macaroon};
use serde::{Deserialize, Serialize};

use crate::config::secret::SecretMacaroon;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Config {
    LndRest(LndRest),
}

impl Config {
    pub fn examples() -> HashMap<String, Self> {
        HashMap::from([("lnd_rest".to_string(), Self::LndRest(LndRest::example()))])
    }

    pub fn build(self) -> anyhow::Result<Box<dyn MerchantApi>> {
        let client = match self {
            Config::LndRest(lnd_rest) => Box::new(lnd_rest.build()?),
        };
        Ok(client)
    }
}

impl super::Secret for Config {
    fn inject(&mut self, prefix: &str) {
        match self {
            Config::LndRest(lnd) => lnd.inject(prefix),
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        match self {
            Config::LndRest(lnd) => lnd.extract(prefix, env_list),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LndRest {
    pub url: String,
    pub macaroon: super::SecretMacaroon,
    pub tls_cert: Option<super::SecretBytes>,
}

impl super::Secret for LndRest {
    fn inject(&mut self, prefix: &str) {
        self.macaroon.inject(&format!("{}_MACAROON", prefix));
        if let Some(ref mut tls) = self.tls_cert {
            tls.inject(&format!("{}_TLS", prefix));
        }
    }

    fn extract(&mut self, prefix: &str, env_list: &mut Vec<String>) {
        self.macaroon
            .extract(&format!("{}_MACAROON", prefix), env_list);
        if let Some(ref mut tls) = self.tls_cert {
            tls.extract(&format!("{}_TLS", prefix), env_list);
        }
    }
}

impl LndRest {
    pub fn example() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            macaroon: SecretMacaroon::new(Macaroon::from("macaroon".as_bytes())),
            tls_cert: None,
        }
    }

    pub fn to_bln_client_config(self) -> bln_client::lnd::Config {
        let block_time = Duration::from_secs(10 * 60);
        let min_cltv = 80;
        let macaroon = self
            .macaroon
            .inner
            .expect("Macaroon required. None provided");
        let max_cache_size = 1000;
        bln_client::lnd::Config::new(
            self.url,
            macaroon,
            block_time,
            min_cltv,
            None,
            max_cache_size,
        )
    }

    pub fn build(self) -> anyhow::Result<bln_client::lnd::Client> {
        let res = bln_client::lnd::Client::try_from(self.to_bln_client_config())?;
        Ok(res)
    }
}
