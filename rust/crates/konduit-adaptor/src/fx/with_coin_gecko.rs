use async_trait::async_trait;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

use crate::fx::interface::{BaseCurrency, Fx, FxError, FxInterface};

#[derive(Debug, Clone, clap::Args)]
pub struct CoinGeckoArgs {
    /// The path to the database file
    #[clap(long, env = crate::env::FX_TOKEN)]
    pub coin_gecko_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WithCoinGecko {
    token: Option<String>,
}

impl TryFrom<&CoinGeckoArgs> for WithCoinGecko {
    type Error = FxError;

    fn try_from(value: &CoinGeckoArgs) -> Result<Self, Self::Error> {
        Ok(WithCoinGecko::new(value.coin_gecko_token.clone()))
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
        let base = BaseCurrency::Eur;
        let coins = with_curl(&base, &self.token).await?;

        let price_map: HashMap<String, f64> = coins
            .into_iter()
            .map(|coin| (coin.id, coin.current_price))
            .collect();

        let ada = *price_map
            .get("cardano")
            .ok_or(FxError::InvalidData("Expect cardano".to_string()))?;
        let bitcoin = *price_map
            .get("bitcoin")
            .ok_or(FxError::InvalidData("Expect bitcoin".to_string()))?;

        let response = Fx::new(base, ada, bitcoin);

        Ok(response)
    }
}

#[derive(Deserialize, Debug)]
struct CoinMarket {
    id: String,
    current_price: f64,
}

/// Requests via Reqwests seem to fail. Via curl succeed some times.
async fn with_curl(
    base: &BaseCurrency,
    token: &Option<String>,
) -> Result<Vec<CoinMarket>, FxError> {
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/markets?vs_currency={base}&ids=bitcoin,cardano"
    );
    let mut output = Command::new("curl");
    output.arg("-s").arg(url);
    if let Some(token) = token {
        output
            .arg("-H")
            .arg(format!("x_cg_demo_api_key : {}", token));
    };
    let output = output.output().map_err(FxError::Io)?;
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
#[allow(dead_code)]
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
