use konduit_data::Duration;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Data that changes frequently and is fully recoverable if lost:
/// `credential` re-issues via `reg`, `pay_request` just means re-quoting.
/// Safe to drop and rebuild at any time.
#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Cache {
    /// Opaque cbor bytes: the last quoted payment request, in whatever
    /// scheme `L2`'s API is currently built around (bolt11 today).
    #[n(0)]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pay_request: Option<Vec<u8>>,
    /// The last time a squash proposal was actually received from the
    /// server. Used by `SquashPolicy::RejectOld` to reject any unlocked
    /// predating this cutoff, guarding against replaying a stale
    /// proposal as current.
    #[n(1)]
    last_received: Option<Duration>,
}

impl Cache {
    pub fn pay_request(&self) -> Option<Vec<u8>> {
        self.pay_request.clone()
    }

    pub fn set_pay_request(&mut self, pay_request: Vec<u8>) {
        self.pay_request = Some(pay_request);
    }

    pub fn clear_pay_request(&mut self) {
        self.pay_request = None;
    }

    pub fn last_received(&self) -> Option<Duration> {
        self.last_received
    }

    pub fn set_last_received(&mut self, at: Duration) {
        self.last_received = Some(at)
    }
}
