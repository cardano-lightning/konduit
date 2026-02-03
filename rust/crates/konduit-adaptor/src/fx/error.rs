use actix_web::{HttpResponse, ResponseError, http::StatusCode};

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

    #[error("Other error")]
    Other(String),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            // Client-side / Validation issues
            Error::InvalidData(_) => StatusCode::BAD_REQUEST,
            Error::ApiError { status, .. } if *status == 404 => StatusCode::NOT_FOUND,

            // Everything else is usually a "server" failure
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        // You can return plain text or a structured JSON error here
        HttpResponse::build(self.status_code()).json(serde_json::json!({
            "status": "error",
            "message": self.to_string()
        }))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
