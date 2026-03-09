use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub index: u64,
    pub amount: u64,
    pub relative_timeout: u64,
    pub routing_fee: u64,
}
