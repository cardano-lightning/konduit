use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlnError {
    #[error("Initialization failed: {0}")]
    Initialization(String),

    #[error("Network or HTTP error: {0}")]
    Network(#[from] reqwest::Error),

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

pub type InvoiceLike = String;

#[derive(Debug, Clone)]
pub struct QuoteResponse {
    pub amount_msats: u64,
    pub recipient: [u8; 33],
    pub payment_hash: [u8; 32],
    pub payment_secret: [u8; 32],
    pub routing_fee: u64,
    pub expiry: u64,
}

#[derive(Debug, Clone)]
pub struct PayRequest {
    pub amount_msats: u64,
    pub recipient: [u8; 33],
    pub payment_hash: [u8; 32],
    pub payment_secret: [u8; 32],
    pub routing_fee: u64,
    pub expiry: u64, // FIXME :: What is this?!
}

#[derive(Debug, Clone)]
pub struct PayResponse {
    pub secret: [u8; 32],
}

#[async_trait]
pub trait BlnInterface: Send + Sync {
    /// Get a quote for paying an invoice.
    async fn quote(&self, invoice_like: InvoiceLike) -> Result<QuoteResponse, BlnError>;

    /// Pay based on a previous quote.
    async fn pay(&self, req: PayRequest) -> Result<PayResponse, BlnError>;
}
