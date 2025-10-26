use async_trait::async_trait;
use thiserror::Error;

use chrono::Utc;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Error)]
pub enum FxError {
    #[error("Process error")]
    Io(#[from] std::io::Error),

    #[error("Network or HTTP error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API returned an error (Status: {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Failed to parse API response: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Data conversion error: {0}")]
    Conversion(#[from] std::array::TryFromSliceError),

    #[error("Other error")]
    Other(String),
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BaseCurrency {
    Eur,
}

impl fmt::Display for BaseCurrency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eur => write!(f, "eur"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Fx {
    pub created_at: i64,
    pub base: BaseCurrency,
    pub ada: f64,
    pub bitcoin: f64,
}

impl Fx {
    pub fn new(base: BaseCurrency, ada: f64, bitcoin: f64) -> Self {
        Fx {
            created_at: Utc::now().timestamp(),
            base,
            ada,
            bitcoin,
        }
    }
}

#[async_trait]
pub trait FxInterface: Send + Sync {
    async fn get(&self) -> Result<Fx, FxError>;
}
