//! The consumer side of the cln rail: the little that is cln's own.
//!
//! A rail carries a payment onward, and originating one is not a rail's
//! job — so this is not a client of its own. It is the instruction a cln
//! rail is written with, the rail's name, and the payme request that is
//! made to the payee rather than to the konduit. Everything else is
//! `l2`'s, which knows no rail.
//!
//! The instruction encodes as JSON because [`crate::Adaptor`] speaks
//! JSON; a konduit reads the instruction back in whichever codec the
//! request arrived in.

use crate::core::{Payme, PaymeRequest, VerifyingKey};
use anyhow::anyhow;
use http_client::{Transport, codec};
use konduit_rail::Rail;

pub use cln_rail::Outbound;

/// The name the rail registers under, taken from the rail itself so the
/// two cannot drift.
pub const RAIL: &str = <cln_rail::Ctx as Rail>::NAME;

/// What the rail is instructed with: who is paid, and how much they
/// receive. The peer's own fee is added by the rail on top of this.
pub fn instruction(payee: VerifyingKey, amount: u64) -> anyhow::Result<Vec<u8>> {
    serde_json::to_vec(&Outbound::new(payee, amount)).map_err(|err| anyhow!(err))
}

/// Asks a payee for a payme. Unauthenticated and over no channel: the
/// payee is not the konduit this client has one with, so it takes a
/// client of its own.
pub async fn request_payme<T: Transport>(
    payee: &http_client::Client<T, codec::Json>,
    amount: u64,
) -> anyhow::Result<Payme>
where
    T::Error: Into<anyhow::Error>,
{
    payee
        .post::<PaymeRequest, Payme>("/pay/konduit/payme", &PaymeRequest { amount })
        .await
        .map_err(|err| anyhow!(err))
}
