use std::sync::Arc;

use crate::{Api, cli::args::Transport, lnd, lnd_rpc, mock};

/// Internal configuration enum representing the chosen backend and its settings.
pub enum Config {
    Mock,
    LndRest(lnd::Config),
    LndRpc(lnd_rpc::Config),
}

impl Config {
    /// Maps the parsed CLI arguments to a specific Config variant based on which flags are present.
    pub fn from_args(args: super::Args) -> Result<Self, String> {
        // Detect if LND is intended by checking for the presence of required LND fields
        if args.mock {
            Ok(Config::Mock)
        } else if let (Some(base_url), Some(macaroon)) = (args.lnd_base_url, args.lnd_macaroon) {
            if Some(Transport::Rpc) == args.lnd_type {
                Ok(Config::LndRpc(lnd_rpc::Config::new(
                    base_url,
                    args.lnd_tls,
                    macaroon,
                    args.block_time,
                    84,
                )))
            } else {
                Ok(Config::LndRest(lnd::Config::new(
                    base_url,
                    args.lnd_tls,
                    macaroon,
                    args.block_time,
                    84,
                    // FIXME :: This may be insufficient in some contexts
                    // It should be double the server's capacity.
                    1000,
                )))
            }
        } else {
            Err("Missing required LND configuration (base URL and Macaroon).".to_string())
        }
    }

    /// Consumes the config and initializes the appropriate API client.
    pub async fn build(self) -> crate::Result<Arc<dyn Api>> {
        match self {
            Config::Mock => {
                let client = mock::Client::new();
                Ok(Arc::new(client))
            }
            Config::LndRest(config) => {
                let client = lnd::Client::try_from(config)?;
                Ok(Arc::new(client))
            }
            Config::LndRpc(config) => {
                let client = lnd_rpc::Client::new(config).await?;
                Ok(Arc::new(client))
            }
        }
    }
}
