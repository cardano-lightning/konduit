//! Consumer-side instruction handling for the BLN rail.

use bln_dto::CommitInstruction;
use konduit_l2_dto::{pay, quote as payment_quote};
use konduit_rail::Rail;

pub use bln_dto::QuoteInstruction;

pub const RAIL: &str = bln_rail::Bln::NAME;

/// Encodes the invoice that the BLN rail will quote against.
pub fn quote_instruction(invoice: String) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&QuoteInstruction { invoice }).map_err(anyhow::Error::from)
}

/// Encodes the invoice that the BLN rail will pay.
pub fn commit_instruction(invoice: String) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&CommitInstruction { invoice }).map_err(anyhow::Error::from)
}

/// Submits the BLN-specific payment instruction through the generic L2 client.
pub async fn pay<T: http_client::Transport>(
    l2: &crate::l2::Client<'_, T>,
    lock: konduit_core::Lock,
    quote: &payment_quote::Response,
    instruction: Vec<u8>,
) -> anyhow::Result<pay::Response>
where
    T::Error: Into<anyhow::Error>,
{
    l2.pay(RAIL, lock, quote, instruction).await
}
