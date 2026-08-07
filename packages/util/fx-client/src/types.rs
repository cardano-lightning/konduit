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

    pub fn msat_to_lovelace(&self, amount: u64) -> u64 {
        (amount as f64 * self.bitcoin / (self.ada * 100_000.0)) as u64
    }

    pub fn lovelace_to_msat(&self, amount: u64) -> u64 {
        ((amount as f64 * self.ada * 100_000.0) / self.bitcoin) as u64
    }
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
