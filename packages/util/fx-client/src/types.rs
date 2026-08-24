use std::{collections::BTreeMap, fmt};

use chrono::Utc;
use minicbor::{Decode, Encode};
use serde::Serialize;

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedRequest {
    pub key: String,
    pub coin_id: String,
}

#[derive(Debug, Clone, Serialize, Encode, Decode)]
pub struct State {
    #[n(0)]
    pub created_at: i64,
    #[n(1)]
    pub base: BaseCurrency,
    #[n(2)]
    pub ada: f64,
    #[n(3)]
    pub bitcoin: f64,
    pub assets: BTreeMap<String, f64>,
}

impl State {
    pub fn new(base: BaseCurrency, ada: f64, bitcoin: f64, assets: BTreeMap<String, f64>) -> Self {
        Self {
            created_at: Utc::now().timestamp(),
            base,
            ada,
            bitcoin,
            assets,
        }
    }

    pub fn asset_usd(&self, key: &str) -> Result<f64> {
        let price = *self
            .assets
            .get(key)
            .ok_or_else(|| Error::InvalidData(format!("missing asset price for {key}")))?;
        valid_price(price, "asset")
    }

    pub fn msat_to_asset_units(
        &self,
        amount_msat: u64,
        decimals: u8,
        asset_usd: f64,
    ) -> Result<u64> {
        let bitcoin = valid_price(self.bitcoin, "bitcoin")?;
        let asset = valid_price(asset_usd, "asset")?;
        let scale = scale(decimals)?;
        checked_floor(amount_msat as f64 * bitcoin * scale as f64 / (100_000_000_000.0 * asset))
    }

    pub fn asset_units_to_msat(&self, amount: u64, decimals: u8, asset_usd: f64) -> Result<u64> {
        let bitcoin = valid_price(self.bitcoin, "bitcoin")?;
        let asset = valid_price(asset_usd, "asset")?;
        let scale = scale(decimals)?;
        checked_floor(amount as f64 * 100_000_000_000.0 * asset / (bitcoin * scale as f64))
    }
}

fn valid_price(price: f64, label: &str) -> Result<f64> {
    if price.is_finite() && price > 0.0 {
        Ok(price)
    } else {
        Err(Error::InvalidData(format!(
            "{label} price must be finite and positive"
        )))
    }
}

fn scale(decimals: u8) -> Result<u64> {
    10_u64
        .checked_pow(decimals.into())
        .ok_or_else(|| Error::InvalidData("asset decimal scale overflows u64".into()))
}

fn checked_floor(value: f64) -> Result<u64> {
    let value = value.floor();
    if value.is_finite() && value >= 0.0 && value < 18_446_744_073_709_551_616.0 {
        Ok(value as u64)
    } else {
        Err(Error::InvalidData(
            "converted amount is outside the u64 range".into(),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum BaseCurrency {
    Aud,
    Chf,
    Eur,
    Gbp,
    Jpy,
    Usd,
}

impl fmt::Display for BaseCurrency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Aud => write!(f, "aud"),
            Self::Chf => write!(f, "chf"),
            Self::Eur => write!(f, "eur"),
            Self::Gbp => write!(f, "gbp"),
            Self::Jpy => write!(f, "jpy"),
            Self::Usd => write!(f, "usd"),
        }
    }
}

impl std::str::FromStr for BaseCurrency {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aud" => Ok(Self::Aud),
            "chf" => Ok(Self::Chf),
            "eur" => Ok(Self::Eur),
            "gbp" => Ok(Self::Gbp),
            "jpy" => Ok(Self::Jpy),
            "usd" => Ok(Self::Usd),
            _ => Err(format!("'{s}' is not a valid base currency")),
        }
    }
}

impl<C> minicbor::Encode<C> for BaseCurrency {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.str(&self.to_string())?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for BaseCurrency {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        d.str()?
            .parse()
            .map_err(|_: String| minicbor::decode::Error::message("invalid base currency"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State::new(
            BaseCurrency::Usd,
            0.5,
            100_000.0,
            BTreeMap::from([("custom".into(), 2.0)]),
        )
    }

    #[test]
    fn converts_six_decimal_assets_both_directions() {
        let state = state();
        assert_eq!(
            state.msat_to_asset_units(100_000_000, 6, 1.0).unwrap(),
            100_000_000
        );
        assert_eq!(
            state.msat_to_asset_units(100_000_000, 6, 2.0).unwrap(),
            50_000_000
        );
        assert_eq!(
            state.asset_units_to_msat(50_000_000, 6, 2.0).unwrap(),
            100_000_000
        );
        assert_eq!(state.asset_usd("custom").unwrap(), 2.0);
    }

    #[test]
    fn rejects_invalid_prices_scales_and_results() {
        for price in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(state().msat_to_asset_units(1, 6, price).is_err());
        }
        let mut invalid_btc = state();
        invalid_btc.bitcoin = 0.0;
        assert!(invalid_btc.msat_to_asset_units(1, 6, 1.0).is_err());
        assert!(state().msat_to_asset_units(1, 20, 1.0).is_err());
        assert!(state().asset_units_to_msat(u64::MAX, 0, f64::MAX).is_err());
        assert!(state().asset_usd("missing").is_err());
    }
}
