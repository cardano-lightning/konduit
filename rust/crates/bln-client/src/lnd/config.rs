use std::time::Duration;

/// FIXME :: [NOTE ON TIME]
/// The average block is ~10 minutes = 600seconds.
/// However, this is probablistic, and is subject to parameters that change every 2016 blocks.
/// if final ctlv is 80 and each hop is 40 this is a very long hold period.
/// This is an estimate. See also risk assessment temporal divergence

#[derive(Debug)]
pub struct Config {
    pub base_url: String,
    pub macaroon: Macaroon,
    pub block_time: Duration,
    pub min_cltv: u64,
    pub tls_certificate: Option<Vec<u8>>,
    pub max_cache_size: usize,
}

impl Config {
    pub fn new(
        base_url: String,
        macaroon: Macaroon,
        block_time: Duration,
        min_cltv: u64,
        tls_certificate: Option<Vec<u8>>,
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

#[derive(Debug, Clone)]
pub struct Macaroon(Vec<u8>);

impl std::str::FromStr for Macaroon {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        hex::decode(s).map_err(|err| err.to_string()).map(Macaroon)
    }
}

impl AsRef<[u8]> for Macaroon {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
