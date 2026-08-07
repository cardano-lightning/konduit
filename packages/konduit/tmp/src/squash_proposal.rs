use konduit_data::{Locked, Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SquashProposal {
    #[n(0)]
    pub proposal: SquashBody,
    #[n(1)]
    pub current: Squash,
    #[n(2)]
    pub unlockeds: Vec<Unlocked>,
    #[n(3)]
    pub lockeds: Vec<Locked>,
}
