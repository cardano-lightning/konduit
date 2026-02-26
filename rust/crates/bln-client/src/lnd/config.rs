use std::time::Duration;

use crate::{macaroon::Macaroon, tls_certificate::TlsCertificate};

/// FIXME :: [NOTE ON TIME]
/// The average block is ~10 minutes = 600seconds.
/// However, this is probablistic, and is subject to parameters that change every 2016 blocks.
/// if final ctlv is 80 and each hop is 40 this is a very long hold period.
/// This is an estimate. See also risk assessment temporal divergence

#[derive(Debug)]
pub struct Config {
    pub base_url: String,
    pub tls_certificate: Option<TlsCertificate>,
    pub macaroon: Macaroon,
    pub block_time: Duration,
    pub min_cltv: u64,
    pub max_cache_size: usize,
}

impl Config {
    pub fn new(
        base_url: String,
        tls_certificate: Option<TlsCertificate>,
        macaroon: Macaroon,
        block_time: Duration,
        min_cltv: u64,
        max_cache_size: usize,
    ) -> Self {
        Self {
            base_url: base_url.trim_end_matches("/").to_string(),
            macaroon,
            block_time,
            min_cltv,
            tls_certificate,
            max_cache_size,
        }
    }
}
