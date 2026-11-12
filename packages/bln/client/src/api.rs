use crate::types::{
    PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse,
};
use async_trait::async_trait;

/// Read-only node status shared across Lightning implementations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Health {
    pub backend: String,
    pub identity_pubkey: Option<String>,
    pub alias: Option<String>,
    pub network: Option<String>,
    pub block_height: Option<u32>,
    pub synced_to_chain: Option<bool>,
    pub synced_to_graph: Option<bool>,
    pub num_peers: Option<u32>,
    pub num_active_channels: Option<u32>,
}

/// Parameters for creating a fixed-amount Lightning invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateInvoiceRequest {
    pub amount_msat: u64,
    pub description: String,
}

/// A newly generated BOLT11 payment request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreatedInvoice {
    pub bolt11: String,
}

#[allow(clippy::double_must_use)] // async_trait adds must_use to its generated Future return type.
#[async_trait]
pub trait Api: Send + Sync {
    /// Query node status. This is read-only and suitable for health checks.
    async fn health(&self) -> crate::Result<Health>;

    /// Create an invoice for receiving a fixed amount.
    async fn create_invoice(&self, request: CreateInvoiceRequest) -> crate::Result<CreatedInvoice>;

    /// Get a quote for paying an invoice.
    async fn quote(&self, quote_request: QuoteRequest) -> crate::Result<QuoteResponse>;

    /// Commit a previously quoted payment. Implementations cap fees at the quote's fee.
    async fn commit(&self, req: PayRequest) -> crate::Result<PayResponse>;

    /// Reveal a secret if it is known
    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse>;
}

pub(crate) fn validate_commit_quote(
    backend: &str,
    quote: &QuoteResponse,
    invoice: &crate::types::Invoice,
) -> crate::Result<()> {
    if quote.backend != backend
        || quote.amount_msat != invoice.amount_msat
        || quote.payee != invoice.payee_compressed.serialize()
    {
        return Err(crate::Error::InvalidData(
            "quote does not match this backend and invoice".into(),
        ));
    }
    Ok(())
}
