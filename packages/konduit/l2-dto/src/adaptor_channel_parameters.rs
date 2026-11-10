use konduit_core::{Duration, VerifyingKey};
#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Adaptor policy advertised by `/info`; distinct from on-chain `Constants`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct AdaptorChannelParameters {
    #[cfg_attr(feature = "serde", serde(with = "konduit_core::serde::bytes"))]
    #[cfg_attr(feature = "cbor", cbor(n(0)))]
    pub adaptor_key: VerifyingKey,
    #[cfg_attr(feature = "cbor", n(1))]
    pub close_period: Duration,
    #[cfg_attr(feature = "cbor", n(2))]
    pub tag_length: usize,
}
