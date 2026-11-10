use crate::{Locked, Squash, SquashBody, Unlocked};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(minicbor::Encode, minicbor::Decode)]
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
