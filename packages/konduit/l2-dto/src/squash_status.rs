use konduit_core::SquashProposal;
#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub enum SquashStatus {
    /// Consumer up-to-date
    #[cfg_attr(feature = "cbor", n(0))]
    Complete,
    /// Something to squash
    #[cfg_attr(feature = "cbor", n(1))]
    Incomplete(#[cfg_attr(feature = "cbor", n(0))] SquashProposal),
    /// Consumer not up-to-date, but nothing to squash
    #[cfg_attr(feature = "cbor", n(2))]
    Stale(#[cfg_attr(feature = "cbor", n(0))] SquashProposal),
}
