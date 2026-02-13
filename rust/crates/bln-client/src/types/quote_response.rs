use std::time::Duration;

#[derive(Debug, Clone)]
pub struct QuoteResponse {
    pub relative_timeout: Duration,
    pub fee_msat: u64,
}
