use std::time::Duration;

use crate::macaroon::Macaroon;
use crate::tls_certificate::TlsCertificate;

/// Flat structure for backend client configuration.
#[derive(Debug, clap::Args)]
pub struct ClientArgs {
    /// BLN block time specified independently of the specific client implementation.
    #[arg(
        long,
        env = "BLN_BLOCK_TIME",
        value_parser = humantime::parse_duration,
        default_value = "10m",
        global = true
    )]
    #[cfg_attr(feature = "namespaced", arg(long("bln-block-time")))]
    pub block_time: Duration,

    /// When set, run commands against a mocked LND server.
    #[arg(long, env = "BLN_MOCK", default_value_t = false)]
    #[cfg_attr(feature = "namespaced", arg(long("bln-mock")))]
    pub mock: bool,

    /// The base URL of the LND REST API.
    #[arg(long, env = "LND_TYPE")]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-type")))]
    pub lnd_type: Option<Transport>,

    /// The base URL of the LND REST API.
    #[arg(long, env = "LND_BASE_URL")]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-base-url")))]
    pub lnd_base_url: Option<String>,

    /// LND TLS in base64 format
    #[arg(long, env = "LND_TLS")]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-tls")))]
    pub lnd_tls: Option<TlsCertificate>,

    /// LND Macaroon in hex format. Pulled from LND_MACAROON env var.
    #[arg(long, env = "LND_MACAROON", hide_env_values = true)]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-macaroon")))]
    pub lnd_macaroon: Option<Macaroon>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Transport {
    /// Use REST API
    Rest,
    /// Use RPC
    Rpc,
}
