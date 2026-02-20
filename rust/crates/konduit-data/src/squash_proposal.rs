use crate::{Locked, Squash, SquashBody, Unlocked};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SquashProposal {
    pub proposal: SquashBody,
    pub current: Option<Squash>,
    pub unlockeds: Vec<Unlocked>,
    pub lockeds: Vec<Locked>,
}
