use crate::{Api, BaseCurrency, binance, coin_gecko, fixed, kraken};
use clap::Parser;
use std::time::Duration;
use tokio::time::interval;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, env = "FX_EVERY", value_parser = humantime::parse_duration, default_value = "2s")]
    pub every: Duration,
    #[arg(long, env = "FX_BASE_CURRENCY", value_enum, default_value = "eur")]
    pub base_currency: BaseCurrency,
    #[arg(long, env = "FX_BINANCE", default_value_t = false)]
    pub binance: bool,
    #[arg(long, env = "FX_COIN_GECKO_TOKEN")]
    pub coin_gecko_token: Option<String>,
    #[arg(long, env = "FX_COIN_GECKO", default_value_t = false)]
    pub coin_gecko_public: bool,
    #[arg(long, env = "FX_BITCOIN")]
    pub bitcoin: Option<f64>,
    #[arg(long, env = "FX_ADA")]
    pub ada: Option<f64>,
    #[arg(long, env = "FX_KRAKEN", default_value_t = false)]
    pub kraken: bool,
}

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
    pub fn from_args(args: Args) -> Option<Self> {
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
