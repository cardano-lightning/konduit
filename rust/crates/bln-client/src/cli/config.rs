use std::sync::Arc;

use crate::{Api, lnd, mock};

/// Internal configuration enum representing the chosen backend and its settings.
pub enum Config {
    Mock,
    Lnd(lnd::Config),
}

impl Config {
    /// Maps the parsed CLI arguments to a specific Config variant based on which flags are present.
    pub fn from_args(args: super::Args) -> Result<Self, String> {
        // Detect if LND is intended by checking for the presence of required LND fields
        if args.mock {
            Ok(Config::Mock)
        } else if let (Some(base_url), Some(macaroon)) = (args.lnd_base_url, args.lnd_macaroon) {
            Ok(Config::Lnd(lnd::Config {
                base_url,
                macaroon,
                block_time: args.block_time,
                min_cltv: 84,
                tls_certificate: None,
            }))
        } else {
            Err("Missing required LND configuration (base URL and Macaroon).".to_string())
        }
    }

    /// Consumes the config and initializes the appropriate API client.
    pub fn build(self) -> crate::Result<Arc<dyn Api>> {
        match self {
            Config::Lnd(config) => {
                let client = lnd::Client::try_from(config)?;
                Ok(Arc::new(client))
            }
            Config::Mock => {
                let client = mock::Client::new();
                Ok(Arc::new(client))
            }
        }
    }
}
