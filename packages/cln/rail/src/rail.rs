//! `cln` as an outbound rail.
//!
//! CLN is the Cardano Lightning Network, so this rail's hop is a konduit
//! channel and its backend is a konduit node: the rail is a konduit
//! client pointed downstream. Its target is therefore the payee's key,
//! and everything else — the channel, the cheque, the squash — is its
//! own.
//!
//! The rail charges nothing and keeps no margin of its own: `Quoted` is
//! what the network downstream costs, passed through. Nor does it hold a
//! volatility buffer, since both sides of this hop are ADA on Cardano's
//! clock and there is nothing to diverge.

use konduit_data::{Lock, VerifyingKey};
use konduit_rail::{Constraints, Outcome, QuoteProblem, Quoted, Rail, Resolution};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::Ctx;

/// This rail's target: who to pay. A konduit verifying key, which for a
/// one-hop rail is also the peer whose channel carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Payee(#[n(0)] pub VerifyingKey);

impl Rail for Ctx {
    const NAME: &'static str = "cln";
    type QuoteTarget = Payee;
    type CommitTarget = Payee;

    async fn quote(&self, amount: u64, target: Payee) -> Result<Quoted, QuoteProblem> {
        self.quote_payee(&target.0, amount).await
    }

    async fn commit(&self, inbound: Constraints, target: Payee) -> Outcome {
        self.commit_payee(inbound, &target.0).await
    }

    async fn reveal(&self, lock: &Lock) -> Resolution {
        self.reveal_lock(lock)
    }
}
