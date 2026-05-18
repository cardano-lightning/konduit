use konduit_data::{Locked, Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SquashProposal {
    #[cbor(n(0), with = "konduit_data::cbor_with::plutus_data")]
    pub proposal: SquashBody,
    #[cbor(n(1), with = "konduit_data::cbor_with::plutus_data")]
    pub current: Squash,
    #[cbor(n(2), with = "konduit_data::cbor_with::vec_plutus_data")]
    pub unlockeds: Vec<Unlocked>,
    #[cbor(n(3), with = "konduit_data::cbor_with::vec_plutus_data")]
    pub lockeds: Vec<Locked>,
}
