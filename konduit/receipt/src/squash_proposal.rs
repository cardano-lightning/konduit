use konduit_data::{Locked, Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// This arrives unverified
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SquashProposal {
    #[n(0)]
    pub proposal: SquashBody,
    #[n(1)]
    pub current: Squash,
    #[n(2)]
    pub unlockeds: Vec<Unlocked>,
    /// This is purely informational
    #[n(3)]
    pub lockeds: Vec<Locked>,
}
