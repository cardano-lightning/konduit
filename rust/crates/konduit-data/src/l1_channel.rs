use serde::{Deserialize, Serialize};

use crate::Stage;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L1Channel {
    pub amount: u64,
    pub stage: Stage,
}
