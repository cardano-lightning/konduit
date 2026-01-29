use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum Error {
    #[error("Init failed: {0}")]
    Init(String),

    #[error("Network or HTTP error: {0}")]
    #[serde(skip_serializing, skip_deserializing)]
    Network(#[from] reqwest::Error),

    #[error("Failed to find the time")]
    Time,

    #[error("Failed to parse API response: {0}")]
    Parse(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("API returned an error (Status: {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Hex decoding error: {0}")]
    #[serde(skip_serializing, skip_deserializing)]
    Hex(#[from] hex::FromHexError),

    #[error("Base64 decoding error: {0}")]
    #[serde(skip_serializing, skip_deserializing)]
    Base64(#[from] base64::DecodeError),

    #[error("Data conversion error: {0}")]
    #[serde(skip_serializing, skip_deserializing)]
    Conversion(#[from] std::array::TryFromSliceError),
}

pub type Result<T> = std::result::Result<T, Error>;
