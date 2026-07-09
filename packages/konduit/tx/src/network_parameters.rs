//! Cache-able network parameters
use cardano_sdk::{NetworkId, ProtocolParameters};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Container of stuff that is "almost constant"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct NetworkParameters {
    #[n(0)]
    pub network_id: NetworkId,
    #[n(1)]
    pub protocol_parameters: ProtocolParameters,
}
