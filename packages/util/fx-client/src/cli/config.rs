use crate::{Api, BaseCurrency, FeedRequest, binance, coin_gecko, fixed, kraken};

#[derive(Debug, Clone)]
pub enum Config {
    Binance {
        base: BaseCurrency,
    },
    CoinGecko {
        base: BaseCurrency,
        token: Option<String>,
    },
    Fixed {
        base: BaseCurrency,
        bitcoin: f64,
        ada: f64,
    },
    Kraken {
        base: BaseCurrency,
    },
}

impl Config {
    pub fn from_args(args: super::Args) -> Option<Self> {
        if let (Some(bitcoin), Some(ada)) = (args.bitcoin, args.ada) {
            return Some(Config::Fixed {
                base: args.base_currency,
                bitcoin,
                ada,
            });
        }
        if args.coin_gecko_token.is_some() || args.coin_gecko_public {
            return Some(Config::CoinGecko {
                token: args.coin_gecko_token,
                base: args.base_currency,
            });
        }
        if args.binance {
            return Some(Config::Binance {
                base: args.base_currency,
            });
        }
        if args.kraken {
            return Some(Config::Kraken {
                base: args.base_currency,
            });
        }
        None
    }

    pub fn build(self, feeds: Vec<FeedRequest>) -> anyhow::Result<Box<dyn Api + Send + Sync>> {
        if !feeds.is_empty() && !matches!(&self, Config::CoinGecko { .. }) {
            return Err(anyhow::anyhow!(
                "configured variable assets require the CoinGecko FX provider"
            ));
        }
        match self {
            Config::Binance { base } => Ok(Box::new(binance::Client::new(base)?)),
            Config::CoinGecko { base, token } => {
                Ok(Box::new(coin_gecko::Client::new(base, token, feeds)))
            }
            Config::Fixed { base, bitcoin, ada } => {
                Ok(Box::new(fixed::Client::new(base, bitcoin, ada)))
            }
            Config::Kraken { base } => Ok(Box::new(kraken::Client::new(base))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed() -> Vec<FeedRequest> {
        vec![FeedRequest {
            key: "custom".into(),
            coin_id: "snek".into(),
        }]
    }

    #[test]
    fn only_coingecko_accepts_variable_asset_feeds() {
        for config in [
            Config::Binance {
                base: BaseCurrency::Usd,
            },
            Config::Kraken {
                base: BaseCurrency::Usd,
            },
            Config::Fixed {
                base: BaseCurrency::Usd,
                bitcoin: 100_000.0,
                ada: 0.5,
            },
        ] {
            assert_eq!(
                config.build(feed()).err().unwrap().to_string(),
                "configured variable assets require the CoinGecko FX provider"
            );
        }
        assert!(
            Config::CoinGecko {
                base: BaseCurrency::Usd,
                token: None,
            }
            .build(feed())
            .is_ok()
        );
    }
}
