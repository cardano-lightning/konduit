use crate::types::{
    PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse,
};
use async_trait::async_trait;

#[allow(clippy::double_must_use)] // async_trait adds must_use to its generated Future return type.
#[async_trait]
pub trait Api: Send + Sync {
    /// Get a quote for paying an invoice.
    async fn quote(&self, quote_request: QuoteRequest) -> crate::Result<QuoteResponse>;

    /// Pay based on a previous quote.
    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse>;

    /// Reveal a secret if it is known
    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse>;
}
