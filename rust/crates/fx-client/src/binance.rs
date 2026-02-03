use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;

use crate::{Api, BaseCurrency, Error, State};

#[derive(Debug, Clone)]
pub struct Client {
    base: BaseCurrency,
}

impl Client {
    /// Creates a new Binance client.
    /// Returns an error if the selected currency is not supported by Binance Spot API.
    pub fn new(base: BaseCurrency) -> anyhow::Result<Self> {
        if matches!(base, BaseCurrency::Chf) {
            return Err(anyhow::anyhow!(
                "CHF is not supported on the Binance Spot API. Try using Kraken or CoinGecko."
            ));
        }
        Ok(Self { base })
    }

    fn get_base_ticker(&self) -> String {
        match self.base {
            BaseCurrency::Usd => "USDT".to_string(),
            _ => self.base.to_string().to_uppercase(),
        }
    }
}

#[async_trait]
impl Api for Client {
    async fn get(&self) -> crate::Result<State> {
        let base_ticker = self.get_base_ticker();

        let btc_symbol = format!("BTC{}", base_ticker);
        let ada_symbol = format!("ADA{}", base_ticker);

        let url = format!(
            "https://api.binance.com/api/v3/ticker/price?symbols=[\"{}\",\"{}\"]",
            btc_symbol, ada_symbol
        );

        let stdout = curl_get_raw(&url).await?;

        // Binance returns a Map {"code": ..., "msg": ...} on error
        if let Ok(api_err) = serde_json::from_slice::<BinanceError>(&stdout) {
            return Err(Error::Other(format!(
                "Binance API Error: {} (code: {})",
                api_err.msg, api_err.code
            )));
        }

        let resp: Vec<BinancePrice> = serde_json::from_slice(&stdout).map_err(|e| {
            let raw = String::from_utf8_lossy(&stdout);
            Error::Other(format!(
                "Failed to parse API response: {}\nRaw response: {}",
                e, raw
            ))
        })?;

        let price_map: std::collections::HashMap<String, f64> = resp
            .into_iter()
            .map(|p| (p.symbol, p.price.parse().unwrap_or(0.0)))
            .collect();

        let btc_price = *price_map
            .get(&btc_symbol)
            .ok_or_else(|| Error::InvalidData(format!("Missing price for {}", btc_symbol)))?;

        let ada_price = *price_map
            .get(&ada_symbol)
            .ok_or_else(|| Error::InvalidData(format!("Missing price for {}", ada_symbol)))?;

        Ok(State::new(self.base.clone(), ada_price, btc_price))
    }
}

#[derive(Deserialize, Debug)]
struct BinancePrice {
    symbol: String,
    price: String,
}

#[derive(Deserialize, Debug)]
struct BinanceError {
    code: i32,
    msg: String,
}

async fn curl_get_raw(url: &str) -> Result<Vec<u8>, Error> {
    let output = Command::new("curl")
        .arg("-g")
        .arg("-s")
        .arg(url)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(format!("Binance curl failed: {}", stderr)));
    }

    Ok(output.stdout)
}
