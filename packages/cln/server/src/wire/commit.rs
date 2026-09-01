use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::{Locked, Secret, VerifyingKey};

pub const PATH: &str = "/ch/x/cln/commit";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Outbound {
    #[n(0)]
    pub key: VerifyingKey,
    #[n(1)]
    pub amount: u64,
}

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
