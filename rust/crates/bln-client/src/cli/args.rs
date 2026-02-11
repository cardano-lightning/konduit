use std::time::Duration;

use crate::lnd;

/// Flat structure for backend client configuration.
#[derive(Debug, clap::Args)]
pub struct ClientArgs {
    /// BLN block time. Defaults to 10 minutes (600s).
    /// Specified independently of the specific client implementation.
    #[arg(long, env = "BLN_BLOCK_TIME", value_parser = humantime::parse_duration, default_value = "10m", global = true)]
    #[cfg_attr(feature = "namespaced", arg(long("bln-block-time")))]
    pub block_time: Duration,

    // Mock
    /// The base URL of the LND REST API.
    #[arg(long, env = "BLN_MOCK", default_value_t = false)]
    #[cfg_attr(feature = "namespaced", arg(long("bln-mock")))]
    pub mock: bool,

    // LND
    /// The base URL of the LND REST API.
    #[arg(long, env = "LND_BASE_URL")]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-base-url")))]
    pub lnd_base_url: Option<String>,

    /// LND Macaroon in hex format. Pulled from LND_MACAROON env var.
    #[arg(long, env = "LND_MACAROON", hide_env_values = true)]
    #[cfg_attr(feature = "namespaced", arg(long("bln-lnd-macaroon")))]
    pub lnd_macaroon: Option<lnd::Macaroon>,
}
