use async_trait::async_trait;

use crate::{PayRequest, PayResponse, QuoteRequest, QuoteResponse};

#[async_trait]
pub trait Api: Send + Sync {
    /// Get a quote for paying an invoice.
    async fn quote(&self, quote_request: QuoteRequest) -> crate::Result<QuoteResponse>;

    /// Pay based on a previous quote.
    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse>;
}
