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

    #[error("Payment failed: {0}")]
    PaymentFailed(String),

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

impl Error {
    pub fn is_terminal_payment_failure(&self) -> bool {
        matches!(
            self,
            Self::PaymentFailed(_)
                | Self::ApiError {
                    status: 400..=499,
                    ..
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn only_definitive_client_rejections_are_terminal() {
        assert!(Error::PaymentFailed("failed".into()).is_terminal_payment_failure());
        assert!(
            Error::ApiError {
                status: 400,
                message: "expired".into()
            }
            .is_terminal_payment_failure()
        );
        assert!(
            !Error::ApiError {
                status: 503,
                message: "unavailable".into()
            }
            .is_terminal_payment_failure()
        );
        assert!(!Error::Time.is_terminal_payment_failure());
    }
}

pub type Result<T> = std::result::Result<T, Error>;
