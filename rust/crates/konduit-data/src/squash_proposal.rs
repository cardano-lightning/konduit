use crate::{Locked, Squash, SquashBody, Unlocked};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquashProposal {
    pub proposal: SquashBody,
    pub current: Squash,
    pub unlockeds: Vec<Unlocked>,
    pub lockeds: Vec<Locked>,
}
