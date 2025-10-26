use serde::{Deserialize, Serialize};

// This does not exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixedReceipt(pub u64);
