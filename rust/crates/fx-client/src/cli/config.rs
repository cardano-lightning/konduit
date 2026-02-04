use crate::{Api, BaseCurrency, binance, coin_gecko, fixed, kraken};

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

    pub fn build(self) -> anyhow::Result<Box<dyn Api + Send + Sync>> {
        match self {
            Config::Binance { base } => Ok(Box::new(binance::Client::new(base)?)),
            Config::CoinGecko { base, token } => Ok(Box::new(coin_gecko::Client::new(base, token))),
            Config::Fixed { base, bitcoin, ada } => {
                Ok(Box::new(fixed::Client::new(base, bitcoin, ada)))
            }
            Config::Kraken { base } => Ok(Box::new(kraken::Client::new(base))),
        }
    }
}
