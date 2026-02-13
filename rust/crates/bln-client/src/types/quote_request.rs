#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub amount_msat: u64,
    pub payee: [u8; 33],
}
