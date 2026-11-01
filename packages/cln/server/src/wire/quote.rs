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

/// FIXME :: There is no seperation between inbound and outbound types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    #[n(0)]
    pub index: u64,
    // Estimated routing fee (**Relative** amount).
    #[n(1)]
    pub fee: u64,
    // **Relative** timeout
    #[n(2)]
    pub timeout: Duration,
}
