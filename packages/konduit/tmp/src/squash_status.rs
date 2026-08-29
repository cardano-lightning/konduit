use crate::SquashProposal;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum SquashStatus {
    /// Consumer up-to-date
    #[n(0)]
    Complete,
    /// Something to squash
    #[n(1)]
    Incomplete(#[n(0)] SquashProposal),
    /// Consumer not up-to-date, but nothing to squash
    #[n(2)]
    Stale(#[n(0)] SquashProposal),
}
