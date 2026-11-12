#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub type Error = crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Request {
    /// Amount requested before rail and Konduit fees.
    #[cfg_attr(feature = "cbor", n(0))]
    pub amount: u64,
    /// Opaque instruction owned and decoded by the selected rail.
    #[cfg_attr(feature = "serde", serde(with = "konduit_core::serde::bytes", default))]
    #[cfg_attr(feature = "cbor", n(1))]
    pub instruction: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Response {
    #[cfg_attr(feature = "cbor", n(0))]
    pub index: u64,
    #[cfg_attr(feature = "cbor", n(1))]
    pub amount: u64,
    #[cfg_attr(feature = "cbor", n(2))]
    pub relative_timeout: u64,
    #[cfg_attr(feature = "cbor", n(3))]
    /// Everything on top of `amount`: the rail's and konduit's.
    pub fee: u64,
}

pub type Result = std::result::Result<Response, Error>;
