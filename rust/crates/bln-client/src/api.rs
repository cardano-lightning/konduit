use async_trait::async_trait;

use crate::{PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse};

#[async_trait]
pub trait Api: Send + Sync {
    /// Get a quote for paying an invoice.
    async fn quote(&self, quote_request: QuoteRequest) -> crate::Result<QuoteResponse>;

    /// Pay based on a previous quote.
    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse>;

    /// Reveal a secret if it is known
    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse>;
}
