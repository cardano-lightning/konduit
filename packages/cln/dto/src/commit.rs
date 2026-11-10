use konduit_core::VerifyingKey;

#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Request {
    #[cfg_attr(feature = "cbor", n(0))]
    pub payee: VerifyingKey,
    #[cfg_attr(feature = "cbor", n(1))]
    pub amount: u64,
}
