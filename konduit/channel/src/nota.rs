//! Everything else that is neither `Backing` nor `Receipt`.
//! (aka "neither of the above")
//! This includes:
//! + (rate) limit,
//! + persistent state between quote and pay
//! + whether the adaptor still serves the channel.

mod limit;
pub use limit::Limit;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Nota {
    #[n(0)]
    limit: Limit,
    #[n(1)]
    quote_bytes: Vec<u8>,
}

impl Nota {
    pub fn new(limit: Limit, quote_bytes: Vec<u8>) -> Self {
        Self { limit, quote_bytes }
    }

    pub fn limit(&self) -> &Limit {
        &self.limit
    }
    pub fn limit_mut(&mut self) -> &mut Limit {
        &mut self.limit
    }
    pub fn quote_bytes(&self) -> &[u8] {
        &self.quote_bytes
    }
    pub fn set_quote_bytes(&mut self, bytes: Vec<u8>) {
        self.quote_bytes = bytes;
    }
}
