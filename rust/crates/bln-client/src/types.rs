use std::time::Duration;

use super::Invoice;

#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub amount_msat: u64,
    pub payee: [u8; 33],
}

#[derive(Debug, Clone)]
pub struct QuoteResponse {
    pub relative_timeout: Duration,
    pub fee_msat: u64,
}

#[derive(Debug, Clone)]
pub struct RevealRequest {
    pub lock: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct RevealResponse {
    pub secret: Option<[u8; 32]>,
}

// Invariant: all fields must match the invoice
// We keep that value at hand so we can provide
// it as a part of the final payment request.
#[derive(Debug, Clone)]
pub struct PayRequest {
    // Max routing fee (msat) that the adaptor is willing to pay
    pub fee_limit: u64,
    /// The relative timeout used to calculate an cltv limit.
    /// The adaptor should have accounted for their margin prioir to this.
    /// In particular, this is not the same value as on the cheque.
    /// This is cheque timeout - adaptor_margin.
    pub relative_timeout: Duration,
    pub invoice: Invoice,
    // /// The following fields are derived from the invoice
    // pub amount_msat: u64,
    // pub payee: [u8; 33],
    // pub payment_hash: [u8; 32],
    // pub payment_secret: [u8; 32],
    // pub final_cltv_delta: u64,
}

pub type PayResponse = RevealResponse;
