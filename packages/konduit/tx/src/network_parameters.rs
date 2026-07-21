//! Cache-able network parameters
use cardano_sdk::{NetworkId, ProtocolParameters};
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Container of stuff that is "almost constant"
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NetworkParameters {
    #[n(0)]
    pub network_id: NetworkId,
    #[n(1)]
    pub protocol_parameters: ProtocolParameters,
}
