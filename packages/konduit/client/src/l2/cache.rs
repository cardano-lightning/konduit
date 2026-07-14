use konduit_wire::reg::cobbl3::Credential;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Data that changes frequently and is fully recoverable if lost:
/// `credential` re-issues via `reg`, `pay_request` just means re-quoting.
/// Safe to drop and rebuild at any time.
#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Cache {
    #[n(0)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    credential: Option<Credential>,
    /// Opaque cbor bytes: the last quoted payment request, in whatever
    /// scheme `L2`'s API is currently built around (bolt11 today).
    #[n(1)]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pay_request: Option<Vec<u8>>,
}

impl Cache {
    pub fn credential(&self) -> Option<Credential> {
        self.credential.clone()
    }
    pub fn set_credential(&mut self, credential: Option<Credential>) {
        self.credential = credential;
    }

    pub fn pay_request(&self) -> Option<Vec<u8>> {
        self.pay_request.clone()
    }
    pub fn set_pay_request(&mut self, pay_request: Vec<u8>) {
        self.pay_request = Some(pay_request);
    }
    pub fn clear_pay_request(&mut self) {
        self.pay_request = None;
    }
}
