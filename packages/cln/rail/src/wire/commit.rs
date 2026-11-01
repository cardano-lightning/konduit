use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::{Locked, Secret};

/// The rail's own commit target, which is what an onward instruction is.
/// One type, so the consumer's instruction and what the rail checks
/// against its ceiling cannot drift apart.
pub use crate::rail::Outbound;

pub const PATH: &str = "/ch/x/cln/commit";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)]
    pub inbound: Locked,
    /// If outbound is None, then assumed self is final payee
    #[n(1)]
    pub outbound: Option<Outbound>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    // None if not resolved but not failed.
    #[n(0)]
    pub secret: Option<Secret>,
}
