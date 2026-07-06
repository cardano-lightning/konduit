#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid base url: {0}")]
    InvalidUrl(String),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("kupo returned an error ({status}): {body}")]
    Api { status: u16, body: String },

    #[error("failed to parse kupo response: {0}")]
    Decode(#[from] serde_json::Error),

    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
