use std::time::Duration;

use crate::macaroon::Macaroon;

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub tls_path: Option<String>,
    pub macaroon: Macaroon,
    pub block_time: Duration,
    pub min_cltv: u64,
}

impl Config {
    pub fn new(
        base_url: String,
        tls_path: Option<String>,
        macaroon: Macaroon,
        block_time: Duration,
        min_cltv: u64,
    ) -> Self {
        Self {
            base_url,
            tls_path,
            macaroon,
            block_time,
            min_cltv,
        }
    }
}
