use minicbor::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Currency {
    #[n(0)]
    Ada,
    #[n(1)]
    Asset(#[n(0)] [u8; 32], #[n(1)] Vec<u8>),
}
