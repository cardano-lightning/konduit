use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlnError {
    #[error("Initialization failed: {0}")]
    Initialization(String),

    #[error("Network or HTTP error: {0}")]
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
    Hex(#[from] hex::FromHexError),

    #[error("Base64 decoding error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Data conversion error: {0}")]
    Conversion(#[from] std::array::TryFromSliceError),
}

#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub amount_msat: u64,
    pub payee: [u8; 33],
}

#[derive(Debug, Clone)]
pub struct QuoteResponse {
    pub estimated_timeout: Duration,
    pub fee_msat: u64,
}

#[derive(Debug, Clone)]
pub struct PayRequest {
    // Max routing fee (msat) that the adaptor is willing to pay
    pub routing_fee: u64,
    /// The max timeout (cltv limit). The adaptor should have accounted for their margin
    /// prioir to this. In other words, this is not the same value as on the cheque.
    /// This is cheque timeout - adaptor_margin.
    pub timeout: Duration,
    /// The following fields are derived from the inovice
    pub amount_msat: u64,
    pub payee: [u8; 33],
    pub payment_hash: [u8; 32],
    pub payment_secret: [u8; 32],
    pub final_cltv_delta: u64,
}

#[derive(Debug, Clone)]
pub struct PayResponse {
    pub secret: [u8; 32],
}

#[async_trait]
pub trait BlnInterface: Send + Sync {
    /// Get a quote for paying an invoice.
    async fn quote(&self, quote_request: QuoteRequest) -> Result<QuoteResponse, BlnError>;

    /// Pay based on a previous quote.
    async fn pay(&self, req: PayRequest) -> Result<PayResponse, BlnError>;
}
