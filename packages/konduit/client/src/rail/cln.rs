//! The consumer side of the cln rail: the little that is cln's own.
//!
//! A rail carries a payment onward, and originating one is not a rail's
//! job — so this is not a client of its own. It is the instruction a cln
//! rail is written with, the rail's name, and the invoice request that is
//! made to the payee rather than to the konduit. Everything else is
//! `l2`'s, which knows no rail.
//!
//! The instruction encodes as JSON because [`crate::Adaptor`] speaks
//! JSON; a konduit reads the instruction back in whichever codec the
//! request arrived in.

use anyhow::anyhow;
use cln_dto::{commit, quote};
use http_client::{Transport, codec};
use konduit_core::VerifyingKey;
use konduit_l2_dto::{pay, quote as payment_quote};
use konduit_rail::Rail;

pub use cln_dto::invoice;

/// The name the rail registers under, taken from the rail itself so the
/// two cannot drift.
pub const RAIL: &str = <cln_rail::Ctx as Rail>::NAME;

/// Encodes the CLN-specific quote instruction for `/pay/cln/quote`.
pub fn quote_instruction(payee: VerifyingKey) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&quote::Request { payee }).map_err(|err| anyhow!(err))
}

/// Encodes the CLN-specific commit instruction for `/pay/cln/pay`.
pub fn commit_instruction(payee: VerifyingKey, amount: u64) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&commit::Request { payee, amount }).map_err(|err| anyhow!(err))
}

/// Asks a payee for an invoice. Unauthenticated and over no channel: the
/// payee is not the konduit this client has one with, so it takes a
/// client of its own.
pub async fn request_invoice<T: Transport>(
    payee: &http_client::Client<T, codec::Json>,
    amount: u64,
) -> anyhow::Result<invoice::Response>
where
    T::Error: Into<anyhow::Error>,
{
    payee
        .post::<invoice::Request, invoice::Response>(
            "/pay/konduit/invoice",
            &invoice::Request { amount },
        )
        .await
        .map_err(|err| anyhow!(err))
}

/// Builds and submits the Konduit cheque for a CLN-specific invoice.
pub async fn pay<T: Transport>(
    l2: &crate::l2::Client<'_, T>,
    rail: &str,
    invoice: &invoice::Response,
    quote: &payment_quote::Response,
    instruction: Vec<u8>,
) -> anyhow::Result<pay::Response>
where
    T::Error: Into<anyhow::Error>,
{
    let now = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_err(|err| anyhow!(err))?
        .as_millis();
    if invoice.timeout.as_millis() <= now {
        return Err(anyhow!("CLN invoice has expired"));
    }

    l2.pay(rail, invoice.lock, quote, instruction).await
}
