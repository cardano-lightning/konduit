use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::{Duration, Lock, VerifyingKey};

pub const PATH: &str = "/ch/x/cln/payme";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)]
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    #[n(0)]
    pub payee: VerifyingKey,
    #[n(1)]
    pub amount: u64,
    #[n(2)]
    pub lock: Lock,
    #[n(3)]
    pub timeout: Duration,
}
