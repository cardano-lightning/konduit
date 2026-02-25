use crate::Stage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L1Channel {
    pub amount: u64,
    pub stage: Stage,
}
