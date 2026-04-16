use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TosInfo {
    #[n(0)]
    pub flat_fee: u64,
}
