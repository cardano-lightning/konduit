use std::sync::Arc;

use crate::{Api, Health, cln, lnd, mock};

/// Internal configuration enum representing the chosen backend and its settings.
pub enum Config {
    Mock,
    Lnd(lnd::Config),
    Cln {
        base_url: String,
        rune: String,
        block_time: std::time::Duration,
    },
}

impl Config {
    /// Maps the parsed CLI arguments to a specific Config variant based on which flags are present.
    pub fn from_args(args: super::Args) -> Result<Self, String> {
        // Detect if LND is intended by checking for the presence of required LND fields
        if args.mock {
            Ok(Config::Mock)
        } else if let (Some(base_url), Some(rune)) = (args.cln_base_url, args.cln_rune) {
            Ok(Config::Cln {
                base_url,
                rune,
                block_time: args.block_time,
            })
        } else if let (Some(base_url), Some(macaroon)) = (args.lnd_base_url, args.lnd_macaroon) {
            Ok(Config::Lnd(lnd::Config::new(
                base_url,
                macaroon,
                args.block_time,
                84,
                None,
                // FIXME :: This may be insufficient in some contexts
                // It should be double the server's capacity.
                1000,
            )))
        } else {
            Err("Provide --mock, both LND settings, or both CLN settings.".to_string())
        }
    }

    pub async fn health(&self) -> crate::Result<Health> {
        match self {
            Config::Mock => mock::Client::new().health().await,
            Config::Lnd(config) => {
                lnd::Client::try_from(lnd::Config {
                    base_url: config.base_url.clone(),
                    macaroon: config.macaroon.clone(),
                    block_time: config.block_time,
                    min_cltv: config.min_cltv,
                    tls_certificate: config.tls_certificate.clone(),
                    max_cache_size: config.max_cache_size,
                })?
                .health()
                .await
            }
            Config::Cln {
                base_url,
                rune,
                block_time,
            } => {
                cln::Client::new(base_url, rune, *block_time)?
                    .health()
                    .await
            }
        }
    }

    /// Consumes the config and initializes the appropriate API client.
    pub fn build(self) -> crate::Result<Arc<dyn Api>> {
        match self {
            Config::Lnd(config) => {
                let client = lnd::Client::try_from(config)?;
                Ok(Arc::new(client))
            }
            Config::Cln {
                base_url,
                rune,
                block_time,
            } => Ok(Arc::new(cln::Client::new(base_url, rune, block_time)?)),
            Config::Mock => {
                let client = mock::Client::new();
                Ok(Arc::new(client))
            }
        }
    }
}
