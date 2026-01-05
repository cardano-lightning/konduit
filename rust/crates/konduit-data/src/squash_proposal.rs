use serde::{Deserialize, Serialize};

use crate::{Squash, SquashBody, Unlocked};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquashProposal {
    pub proposal: SquashBody,
    pub current: Squash,
    pub unlockeds: Vec<Unlocked>,
}
