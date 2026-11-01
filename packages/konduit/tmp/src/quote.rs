use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Quote {
    #[n(0)]
    pub index: u64,
    #[n(1)]
    pub amount: u64,
    #[n(2)]
    pub relative_timeout: u64,
    #[n(3)]
    /// Everything on top of `amount`: the rail's and konduit's.
    pub fee: u64,
}
