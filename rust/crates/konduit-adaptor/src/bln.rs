use std::time::Duration;

use bln_client;
pub use bln_client::{Api, Invoice, PayRequest, QuoteRequest};

#[derive(Debug, Clone, clap::Args)]
pub struct LndArgs {
    /// The base URL of the LND REST API
    #[arg(long, env = "LND_BASE_URL")]
    pub lnd_base_url: String,

    /// LND Macaroon in hex format. Pulled from LND_MACAROON env var.
    #[arg(
        long,
        env = "LND_MACAROON",
        value_parser = |s: &str| hex::decode(s).map_err(|e| e.to_string())
    )]
    pub lnd_macaroon: Vec<u8>,

    /// BLN block time. defaults to 600s = 10 mins
    #[arg(
        long,
        env = "BLN_BLOCK_TIME",
        value_parser = humantime::parse_duration,
        default_value = "10m",
    )]
    pub bln_block_time: Duration,
}

impl From<LndArgs> for bln_client::lnd::Config {
    fn from(value: LndArgs) -> Self {
        bln_client::lnd::Config {
            base_url: value.lnd_base_url,
            macaroon: value.lnd_macaroon,
            block_time: value.bln_block_time,
            min_cltv: 84,
            tls_certificate: None,
        }
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct BlnArgs {
    /// BLN with LND
    #[clap(flatten)]
    pub lnd: Option<LndArgs>,
}

impl BlnArgs {
    pub fn build(self) -> Result<impl bln_client::Api, bln_client::Error> {
        if let Some(args) = &self.lnd {
            let config = bln_client::lnd::Config::from(args.clone());
            let client = bln_client::lnd::Client::try_from(config)?;
            Ok(client)
        } else {
            // Fixme :: error handling
            panic!("BLN cannot start")
        }
    }
}
