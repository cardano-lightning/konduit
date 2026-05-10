use crate::{Locked, Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SquashProposal {
    #[cbor(n(0), with = "crate::cbor_with::plutus_data")]
    pub proposal: SquashBody,
    #[cbor(n(1), with = "crate::cbor_with::plutus_data")]
    pub current: Squash,
    #[cbor(n(2), with = "crate::cbor_with::vec_plutus_data")]
    pub unlockeds: Vec<Unlocked>,
    #[cbor(n(3), with = "crate::cbor_with::vec_plutus_data")]
    pub lockeds: Vec<Locked>,
}
