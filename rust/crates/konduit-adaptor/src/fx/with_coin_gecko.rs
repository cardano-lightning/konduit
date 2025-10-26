use async_trait::async_trait;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

use crate::fx::interface::{BaseCurrency, Fx, FxError, FxInterface};

#[derive(Debug, Clone, clap::Args)]
pub struct CoinGeckoArgs {
    /// The path to the database file
    #[clap(long, env = "KONDUIT_FX_TOKEN")]
    pub coin_gecko_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WithCoinGecko {
    token: Option<String>,
}

impl TryFrom<CoinGeckoArgs> for WithCoinGecko {
    type Error = FxError;

    fn try_from(value: CoinGeckoArgs) -> Result<Self, Self::Error> {
        Ok(WithCoinGecko::new(value.coin_gecko_token))
    }
}

impl WithCoinGecko {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl FxInterface for WithCoinGecko {
    async fn get(&self) -> Result<Fx, FxError> {
        todo!()
    }
}

#[derive(Deserialize, Debug)]
struct CoinMarket {
    id: String,
    current_price: f64,
}

pub async fn get(base: BaseCurrency) -> Result<Fx, FxError> {
    let coins = with_curl().await?;
    let price_map: HashMap<String, f64> = coins
        .into_iter()
        .map(|coin| (coin.id, coin.current_price))
        .collect();

    let ada = price_map
        .get("cardano")
        .ok_or(FxError::InvalidData("Expect cardano".to_string()))?
        .clone();
    let bitcoin = price_map
        .get("bitcoin")
        .ok_or(FxError::InvalidData("Expect bitcoin".to_string()))?
        .clone();

    let response = Fx::new(base, ada, bitcoin);

    Ok(response)
}

/// Requests via Reqwests seem to fail. Via curl succeed some times.
async fn with_curl() -> Result<Vec<CoinMarket>, FxError> {
    let url = "https://api.coingecko.com/api/v3/coins/markets?vs_currency=eur&ids=bitcoin,cardano";
    let output = Command::new("curl")
        .arg("-s")
        .arg(url)
        .output()
        .map_err(FxError::Io)?;
    if output.status.success() {
        // If the API fails, we still only pick this up as a failure to deserialize.
        let response_data: Vec<CoinMarket> =
            serde_json::from_slice(&output.stdout).map_err(FxError::Serde)?;
        Ok(response_data)
    } else {
        let status = output.status;
        let message = String::from_utf8_lossy(&output.stderr);
        Err(FxError::Other(format!(
            "Process failed : {status} : {message}"
        )))
    }
}

// THIS CODE IS IMMEDIATELY RATE LIMITED
async fn with_reqwest(base: BaseCurrency) -> Result<Vec<CoinMarket>, FxError> {
    let params = [
        ("vs_currency", base.to_string()),
        ("ids", "bitcoin,cardano".to_string()),
    ];
    let client = reqwest::Client::new();
    let coins = client
        .get("https://api.coingecko.com/api/v3/coins/markets")
        .query(&params)
        .send()
        .await
        .map_err(FxError::Network)?;
    println!("coins {:?}", coins);
    let coins: Vec<CoinMarket> = coins.json().await?;
    Ok(coins)
}
