#[derive(Debug, thiserror::Error)]
pub enum Error {
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

    #[error("Other error {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
