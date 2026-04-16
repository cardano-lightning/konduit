use cardano_sdk::cbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Encode, Decode, PartialEq, Eq, Hash, Clone)]
pub struct SemVer {
    /// Bumped on any breaking change
    #[n(0)]
    pub major: u16,
    /// Bumped on additive change
    #[n(1)]
    pub minor: u16,
    /// Bumped on bug fix/ patch
    #[n(2)]
    pub patch: u16,
}
