use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::{Duration, VerifyingKey};

pub const PATH: &str = "/ch/x/cln/quote";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)]
    pub payee: VerifyingKey,
    #[n(1)]
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    // Estimated routing fee
    #[n(0)]
    pub fee: u64,
    // Relative timeout
    #[n(1)]
    pub timeout: Duration,
}
