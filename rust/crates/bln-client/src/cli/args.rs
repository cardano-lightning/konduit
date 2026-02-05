use std::time::Duration;

// Re-exports from the client library
pub use crate::{Api, Invoice, PayRequest, PayResponse, QuoteRequest, QuoteResponse, lnd};

/// Flat structure for backend client configuration.
#[derive(Debug, clap::Args)]
pub struct ClientArgs {
    /// BLN block time. Defaults to 10 minutes (600s).
    /// Specified independently of the specific client implementation.
    #[arg(long, env = "BLN_BLOCK_TIME", value_parser = humantime::parse_duration, default_value = "10m", global = true)]
    pub bln_block_time: Duration,

    // LND
    /// The base URL of the LND REST API.
    #[arg(long, env = "LND_BASE_URL")]
    pub lnd_base_url: Option<String>,

    /// LND Macaroon in hex format. Pulled from LND_MACAROON env var.
    #[arg(long, env = "LND_MACAROON")]
    pub lnd_macaroon: Option<lnd::Macaroon>,
}
