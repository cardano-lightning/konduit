use clap::Parser;
use fx_client::{Api, BaseCurrency, binance, coin_gecko, fixed, kraken};
use std::time::Duration;
use tokio::time::interval;

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(
        long,
        env = "FX_EVERY",
        value_parser = humantime::parse_duration,
        default_value = "2s"
    )]
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
pub struct BinanceConfig {
    pub base: BaseCurrency,
}

#[derive(Debug, Clone)]
pub struct CoinGeckoConfig {
    pub base: BaseCurrency,
    pub token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FixedConfig {
    pub base: BaseCurrency,
    pub bitcoin: f64,
    pub ada: f64,
}

#[derive(Debug, Clone)]
pub struct KrakenConfig {
    pub base: BaseCurrency,
}

#[derive(Debug, Clone)]
pub enum Config {
    Binance(BinanceConfig),
    CoinGecko(CoinGeckoConfig),
    Fixed(FixedConfig),
    Kraken(KrakenConfig),
}

impl Config {
    pub fn from_args(args: Args) -> Option<Self> {
        if let (Some(bitcoin), Some(ada)) = (args.bitcoin, args.ada) {
            return Some(Config::Fixed(FixedConfig {
                base: args.base_currency,
                bitcoin,
                ada,
            }));
        }

        if args.coin_gecko_token.is_some() || args.coin_gecko_public {
            return Some(Config::CoinGecko(CoinGeckoConfig {
                token: args.coin_gecko_token,
                base: args.base_currency,
            }));
        }

        if args.binance {
            return Some(Config::Binance(BinanceConfig {
                base: args.base_currency,
            }));
        }

        if args.kraken {
            return Some(Config::Kraken(KrakenConfig {
                base: args.base_currency,
            }));
        }

        None
    }

    pub fn build(self) -> Box<dyn Api + Send + Sync> {
        match self {
            Config::Binance(c) => Box::new(binance::Client::new(c.base).unwrap()),
            Config::CoinGecko(c) => Box::new(coin_gecko::Client::new(c.base, c.token)),
            Config::Fixed(c) => Box::new(fixed::Client::new(c.base, c.bitcoin, c.ada)),
            Config::Kraken(c) => Box::new(kraken::Client::new(c.base)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let args = Args::parse();
    let every = args.every;
    let config = Config::from_args(args).expect("Insufficient arguments to determine a valid FX client. Provide either Fixed (--bitcoin and --ada), CoinGecko (--coin-gecko-token or --coin-gecko-public), or Kraken (--kraken).");
    let client = config.build();
    let mut ticker = interval(every);
    loop {
        ticker.tick().await;
        match client.get().await {
            Ok(output) => println!("Success! Output: {:?}", output),
            Err(e) => eprintln!("Service failed: {}", e),
        }
    }
}
