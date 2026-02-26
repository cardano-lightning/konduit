use std::time::Duration;

use crate::{macaroon::Macaroon, tls_certificate::TlsCertificate};

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub tls: Option<TlsCertificate>,
    pub macaroon: Macaroon,
    pub block_time: Duration,
    pub min_cltv: u64,
}

impl Config {
    pub fn new(
        base_url: String,
        tls: Option<TlsCertificate>,
        macaroon: Macaroon,
        block_time: Duration,
        min_cltv: u64,
    ) -> Self {
        Self {
            base_url,
            tls,
            macaroon,
            block_time,
            min_cltv,
        }
    }
}
