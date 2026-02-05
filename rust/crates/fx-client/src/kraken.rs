use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

use crate::{Api, BaseCurrency, Error, State};

#[derive(Debug, Clone)]
pub struct Client {
    base: BaseCurrency,
}

impl Client {
    pub fn new(base: BaseCurrency) -> Self {
        Self { base }
    }

    /// Determines the pair strings needed for Kraken.
    /// Returns (BitcoinPair, AdaPair, Option<BridgePair>)
    fn get_pairs(&self) -> (String, String, Option<String>) {
        match self.base {
            BaseCurrency::Eur => ("XXBTZEUR".to_string(), "ADAEUR".to_string(), None),
            BaseCurrency::Usd => ("XXBTZUSD".to_string(), "ADAUSD".to_string(), None),
            BaseCurrency::Aud => ("XXBTZAUD".to_string(), "ADAAUD".to_string(), None),
            BaseCurrency::Gbp => ("XXBTZGBP".to_string(), "ADAGBP".to_string(), None),
            // Bridge needed for JPY via USD (ADAUSD * ZUSDZJPY)
            BaseCurrency::Jpy => (
                "XXBTZJPY".to_string(),
                "ADAUSD".to_string(),
                Some("ZUSDZJPY".to_string()),
            ),
            // Bridge needed for CHF via EUR (ADAEUR * EURCHF)
            BaseCurrency::Chf => (
                "XBTCHF".to_string(),
                "ADAEUR".to_string(),
                Some("EURCHF".to_string()),
            ),
        }
    }
}

#[async_trait]
impl Api for Client {
    async fn get(&self) -> crate::Result<State> {
        let (btc_pair, ada_target_pair, bridge_pair) = self.get_pairs();

        let mut pairs_to_fetch = vec![btc_pair.clone(), ada_target_pair.clone()];
        if let Some(ref bridge) = bridge_pair {
            pairs_to_fetch.push(bridge.clone());
        }

        let url = format!(
            "https://api.kraken.com/0/public/Ticker?pair={}",
            pairs_to_fetch.join(",")
        );

        let resp: KrakenResponse = curl_get(&url).await?;

        if !resp.error.is_empty() {
            return Err(Error::Other(format!("Kraken API Error: {:?}", resp.error)));
        }

        let btc_price = parse_kraken_price(&resp.result, &btc_pair)?;

        let ada_price = if let Some(ref bridge) = bridge_pair {
            let ada_intermediate = parse_kraken_price(&resp.result, &ada_target_pair)?;
            let conversion_rate = parse_kraken_price(&resp.result, bridge)?;
            ada_intermediate * conversion_rate
        } else {
            parse_kraken_price(&resp.result, &ada_target_pair)?
        };

        Ok(State::new(self.base.clone(), ada_price, btc_price))
    }
}

fn parse_kraken_price(result: &HashMap<String, KrakenTicker>, key: &str) -> crate::Result<f64> {
    result
        .get(key)
        .ok_or_else(|| Error::InvalidData(format!("Missing pair {} in response", key)))?
        .c
        .first()
        .ok_or_else(|| Error::InvalidData(format!("No price data for {}", key)))?
        .parse::<f64>()
        .map_err(|_| Error::InvalidData(format!("Failed to parse price for {}", key)))
}

#[derive(Deserialize, Debug)]
struct KrakenResponse {
    error: Vec<String>,
    result: HashMap<String, KrakenTicker>,
}

#[derive(Deserialize, Debug)]
struct KrakenTicker {
    /// Last closed trade [price, lot volume]
    c: Vec<String>,
}

async fn curl_get<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, Error> {
    let output = Command::new("curl")
        .arg("-s")
        .arg(url)
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(format!("Curl failed: {}", msg)));
    }

    serde_json::from_slice(&output.stdout).map_err(Error::Serde)
}
