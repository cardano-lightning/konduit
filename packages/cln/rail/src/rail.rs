// FIXME :: do away with this module. rehome content.
//! `cln` as an outbound rail: its two instruction types.
//!
//! CLN is the Cardano Lightning Network, so this rail's hop is a konduit
//! channel and its backend is a konduit node: the rail is a konduit
//! client pointed downstream. Its instructions are written in konduit's
//! own addressing, and everything else — the channel, the cheque, the squash
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
/// own, since konduit passes that separately, so a quote is instructed
/// with less than a commit — as a rail's two instructions may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
// FIXME :: Should serialize as hex/ bytestring.
pub struct Payee(#[n(0)] pub VerifyingKey);

/// What a commit instructs: who to pay, and how much they receive.
///
/// The amount is what reaches the payee, not what leaves this node: the
/// peer downstream is a konduit too and charges to be paid, so the rail
/// adds the peer's own fee on top and checks the sum against
/// `Constraints.amount` — the inbound cheque net of konduit's fee.
/// Whatever is left over is the consumer's own headroom and not
/// something the rail spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Outbound {
    // FIXME :: should be payee
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
