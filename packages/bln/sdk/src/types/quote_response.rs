use std::time::Duration;

#[derive(Debug, Clone)]
pub struct QuoteResponse {
    /// Backend that produced this quote; commit must use the same backend.
    pub backend: String,
    /// Invoice principal in millisatoshis.
    pub amount_msat: u64,
    /// Invoice recipient's compressed public key.
    pub payee: [u8; 33],
    pub relative_timeout: Duration,
    /// Maximum routing fee authorized by this quote, in millisatoshis.
    pub fee_msat: u64,
}
