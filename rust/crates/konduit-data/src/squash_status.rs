use crate::SquashProposal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SquashStatus {
    /// Consumer up-to-date
    Complete,
    /// Something to squash
    Incomplete(SquashProposal),
    /// Consumer not up-to-date, but nothing to squash
    Stale(SquashProposal),
}
