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
