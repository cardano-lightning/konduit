use chrono::Utc;
use minicbor::{Decode, Encode};
use serde::Serialize;
use std::fmt;

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
}

impl State {
    pub fn new(base: BaseCurrency, ada: f64, bitcoin: f64) -> Self {
        State {
            created_at: Utc::now().timestamp(),
            base,
            ada,
            bitcoin,
        }
    }

    /// Convert millisatoshis to lovelace using integer division (truncating
    /// the fractional lovelace). Callers that need a conservative ceiling
    /// can add one lovelace themselves.
    pub fn msat_to_lovelace(&self, amount: u64) -> crate::Result<u64> {
        let ada = scaled_rate(self.ada)?;
        let bitcoin = scaled_rate(self.bitcoin)?;
        let denominator = ada.checked_mul(100_000).ok_or_else(amount_overflow)?;
        let lovelace = u128::from(amount)
            .checked_mul(bitcoin)
            .ok_or_else(amount_overflow)?
            / denominator;
        u64::try_from(lovelace).map_err(|_| amount_overflow())
    }

    /// Convert lovelace to millisatoshis using integer division (truncating
    /// the fractional millisatoshi). Callers that need a conservative ceiling
    /// can add one millisatoshi themselves.
    pub fn lovelace_to_msat(&self, amount: u64) -> crate::Result<u64> {
        let ada = scaled_rate(self.ada)?;
        let bitcoin = scaled_rate(self.bitcoin)?;
        let msat = u128::from(amount)
            .checked_mul(ada)
            .and_then(|value| value.checked_mul(100_000))
            .ok_or_else(amount_overflow)?
            / bitcoin;
        u64::try_from(msat).map_err(|_| amount_overflow())
    }
}

// Rates arrive from upstream APIs as decimal-valued floats. Quantize the
// rate once, then perform all amount arithmetic as checked integers.
const RATE_SCALE: f64 = 100_000_000.0;

fn scaled_rate(rate: f64) -> crate::Result<u128> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err(crate::Error::InvalidData(
            "ADA/bitcoin conversion rate is unavailable or invalid".to_string(),
        ));
    }
    let scaled = rate * RATE_SCALE;
    if !scaled.is_finite() {
        return Err(amount_overflow());
    }
    if scaled <= 0.0 {
        return Err(crate::Error::InvalidData(
            "ADA/bitcoin conversion rate is unavailable or invalid".to_string(),
        ));
    }
    if scaled >= u128::MAX as f64 {
        return Err(amount_overflow());
    }
    let scaled = scaled as u128;
    if scaled == 0 {
        return Err(crate::Error::InvalidData(
            "ADA/bitcoin conversion rate is below supported precision".to_string(),
        ));
    }
    Ok(scaled)
}

fn amount_overflow() -> crate::Error {
    crate::Error::InvalidAmount("converted amount is outside the u64 range".to_string())
}

#[derive(Clone, Serialize, Debug)]
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aud" => Ok(Self::Aud),
            "chf" => Ok(Self::Chf),
            "eur" => Ok(Self::Eur),
            "gbp" => Ok(Self::Gbp),
            "jpy" => Ok(Self::Jpy),
            "usd" => Ok(Self::Usd),
            _ => Err(format!("'{}' is not a valid base currency", s)),
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

    #[test]
    fn ada_bitcoin_conversions_round_trip_with_integer_division() {
        let state = State::new(BaseCurrency::Usd, 2.0, 60_000.0);
        let lovelace = 123_456_789;
        let msat = state.lovelace_to_msat(lovelace).unwrap();
        let round_trip = state.msat_to_lovelace(msat).unwrap();

        assert!(round_trip.abs_diff(lovelace) <= 1);
    }

    #[test]
    fn conversions_truncate_fractional_units_without_implicit_rounding() {
        let state = State::new(BaseCurrency::Usd, 2.0, 60_000.0);
        assert_eq!(state.lovelace_to_msat(1).unwrap(), 3);
        assert_eq!(state.msat_to_lovelace(3).unwrap(), 0);
    }

    #[test]
    fn conversions_fail_when_rate_data_is_unusable_or_result_overflows() {
        let no_rate = State::new(BaseCurrency::Usd, 0.0, 60_000.0);
        assert!(matches!(
            no_rate.lovelace_to_msat(1),
            Err(crate::Error::InvalidData(_))
        ));

        let overflow = State::new(BaseCurrency::Usd, 1.0, f64::MAX);
        assert!(matches!(
            overflow.msat_to_lovelace(u64::MAX),
            Err(crate::Error::InvalidAmount(_))
        ));
    }
}
