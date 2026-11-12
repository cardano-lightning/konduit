use crate::types::Invoice;
use crate::types::QuoteResponse;

/// A payment commit bound to the exact invoice quote shown to the user.
#[derive(Debug, Clone)]
pub struct PayRequest {
    pub quote: QuoteResponse,
    pub invoice: Invoice,
}
