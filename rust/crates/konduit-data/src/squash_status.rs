use crate::SquashProposal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SquashStatus {
    /// Consumer up-to-date
    Complete,
    /// Something to squash
    Incomplete(SquashProposal),
    /// Consumer not up-to-date, but nothing to squash
    Stale(SquashProposal),
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use cardano_sdk::wasm_proxy;
    use serde::{Deserialize, Serialize};

    wasm_proxy! {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        SquashStatus
    }
}
