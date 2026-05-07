use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    /// Cheque index
    #[n(0)]
    pub index: u64,
    /// Cheque amount
    #[n(1)]
    pub amount: u64,
    /// Cheque timeout. Note that this is **relative** and in ms
    #[n(2)]
    pub relative_timeout: u64,
    /// Routing fee. Informational.
    /// Clients should independently calculate the effective fee
    #[n(3)]
    pub fee: u64,
}
