use std::{
    collections::{BTreeMap, HashMap},
    process::Command,
};

use async_trait::async_trait;
use serde::Deserialize;

use crate::{Api, BaseCurrency, Error, FeedRequest, State};
const COINS_PER_PAGE: usize = 250;

#[derive(Debug, Clone)]
pub struct Client {
    token: Option<String>,
    base: BaseCurrency,
    feeds: Vec<FeedRequest>,
}

impl Client {
    pub fn new(base: BaseCurrency, token: Option<String>, feeds: Vec<FeedRequest>) -> Self {
        Self { token, base, feeds }
    }
}

#[async_trait]
impl Api for Client {
    async fn get(&self) -> super::Result<State> {
        let ids = requested_ids(&self.feeds);
        let mut coins = Vec::with_capacity(ids.len());
        for ids in market_id_batches(&ids) {
            coins.extend(with_curl(&self.base, &self.token, &ids).await?);
        }
        state_from_coins(self.base, &self.feeds, coins)
    }
}

fn requested_ids(feeds: &[FeedRequest]) -> Vec<String> {
    let mut ids = vec!["bitcoin".to_owned(), "cardano".to_owned()];
    for feed in feeds {
        if !ids.contains(&feed.coin_id) {
            ids.push(feed.coin_id.clone());
        }
    }
    ids
}

fn market_id_batches(ids: &[String]) -> impl Iterator<Item = String> + '_ {
    ids.chunks(COINS_PER_PAGE).map(|ids| ids.join(","))
}

fn state_from_coins(
    base: BaseCurrency,
    feeds: &[FeedRequest],
    coins: Vec<CoinMarket>,
) -> crate::Result<State> {
    let prices = coins
        .into_iter()
        .map(|coin| (coin.id, coin.current_price))
        .collect::<HashMap<_, _>>();
    let price = |id: &str| {
        let value = *prices
            .get(id)
            .ok_or_else(|| Error::InvalidData(format!("missing CoinGecko price for {id}")))?;
        if value.is_finite() && value > 0.0 {
            Ok(value)
        } else {
            Err(Error::InvalidData(format!(
                "CoinGecko price for {id} must be finite and positive"
            )))
        }
    };
    let bitcoin = price("bitcoin")?;
    let ada = price("cardano")?;
    let assets = feeds
        .iter()
        .map(|feed| Ok((feed.key.clone(), price(&feed.coin_id)?)))
        .collect::<crate::Result<BTreeMap<_, _>>>()?;
    Ok(State::new(base, ada, bitcoin, assets))
}

#[derive(Clone, Deserialize, Debug)]
struct CoinMarket {
    id: String,
    current_price: f64,
}

/// Requests via reqwest are immediately rate-limited in some deployments, so retain curl.
async fn with_curl(
    base: &BaseCurrency,
    token: &Option<String>,
    ids: &str,
) -> Result<Vec<CoinMarket>, Error> {
    let url = market_url(base, ids);
    let mut output = Command::new("curl");
    output.arg("-s").arg(url);
    if let Some(token) = token {
        output.arg("-H").arg(format!("x_cg_demo_api_key : {token}"));
    }
    let output = output.output().map_err(Error::CurlIo)?;
    if output.status.success() {
        serde_json::from_slice(&output.stdout).map_err(Error::Serde)
    } else {
        let status = output.status;
        let message = String::from_utf8_lossy(&output.stderr);
        Err(Error::Other(format!(
            "Process failed : {status} : {message}"
        )))
    }
}

fn market_url(base: &BaseCurrency, ids: &str) -> String {
    format!(
        "https://api.coingecko.com/api/v3/coins/markets?vs_currency={base}&ids={ids}&per_page={COINS_PER_PAGE}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coin(id: &str, current_price: f64) -> CoinMarket {
        CoinMarket {
            id: id.into(),
            current_price,
        }
    }

    #[test]
    fn deduplicates_ids_and_maps_one_price_to_each_alias() {
        let feeds = vec![
            FeedRequest {
                key: "one".into(),
                coin_id: "snek".into(),
            },
            FeedRequest {
                key: "two".into(),
                coin_id: "snek".into(),
            },
        ];
        assert_eq!(requested_ids(&feeds), ["bitcoin", "cardano", "snek"]);
        let state = state_from_coins(
            BaseCurrency::Usd,
            &feeds,
            vec![
                coin("bitcoin", 100_000.0),
                coin("cardano", 0.5),
                coin("snek", 2.0),
            ],
        )
        .unwrap();
        assert_eq!(state.assets["one"], 2.0);
        assert_eq!(state.assets["two"], 2.0);
    }

    #[test]
    fn batches_market_ids_at_the_api_limit() {
        let feeds = (0..249)
            .map(|i| FeedRequest {
                key: i.to_string(),
                coin_id: format!("feed-{i}"),
            })
            .collect::<Vec<_>>();
        let ids = requested_ids(&feeds);
        let batches = market_id_batches(&ids).collect::<Vec<_>>();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].split(',').count(), COINS_PER_PAGE);
        assert_eq!(batches[1], "feed-248");
        assert!(market_url(&BaseCurrency::Usd, &batches[0]).ends_with("per_page=250"));
    }

    #[test]
    fn missing_or_invalid_requested_price_fails_whole_state() {
        let feeds = vec![FeedRequest {
            key: "custom".into(),
            coin_id: "snek".into(),
        }];
        let base = vec![coin("bitcoin", 100_000.0), coin("cardano", 0.5)];
        assert!(state_from_coins(BaseCurrency::Usd, &feeds, base.clone()).is_err());
        assert!(
            state_from_coins(
                BaseCurrency::Usd,
                &feeds,
                base.into_iter().chain([coin("snek", 0.0)]).collect(),
            )
            .is_err()
        );
    }
}
