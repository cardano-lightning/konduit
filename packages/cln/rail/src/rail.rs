//! `cln` as an outbound rail: its two target types.
//!
//! CLN is the Cardano Lightning Network, so this rail's hop is a konduit
//! channel and its backend is a konduit node: the rail is a konduit
//! client pointed downstream. Its targets are therefore konduit's own
//! addressing, and everything else — the channel, the cheque, the squash
//! — is the rail's own.
//!
//! The `impl Rail for Ctx` itself lives in `ctx.rs`, beside the state it
//! works on, so there is one `commit` and no wrapper around it.
//!
//! The rail charges nothing and keeps no margin of its own: `Quoted` is
//! what the network downstream costs, passed through. Nor does it hold a
//! volatility buffer, since both sides of this hop are ADA on Cardano's
//! clock and there is nothing to diverge.

use konduit_data::VerifyingKey;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// What a quote asks about: who to pay. A quote needs no amount of its
/// own — konduit passes that separately — so this is less than a commit
/// target, as a rail's two targets are allowed to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Payee(#[n(0)] pub VerifyingKey);

/// What a commit instructs: who to pay, and how much to pay them.
///
/// The amount is the consumer's instruction, not konduit's ceiling. The
/// rail checks it against `Constraints.amount` — the inbound cheque net
/// of konduit's fee — and commits exactly this much, so the difference
/// between the two is the consumer's own headroom and not something the
/// rail spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Outbound {
    #[n(0)]
    pub key: VerifyingKey,
    #[n(1)]
    pub amount: u64,
}

impl Outbound {
    pub fn new(key: VerifyingKey, amount: u64) -> Self {
        Self { key, amount }
    }
}
