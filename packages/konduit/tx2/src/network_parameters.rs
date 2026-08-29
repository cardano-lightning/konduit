use cardano_sdk::{NetworkId, ProtocolParameters};

/// Container of stuff that is "almost constant"
#[derive(Debug, Clone)]
pub struct NetworkParameters {
    pub network_id: NetworkId,
    pub protocol_parameters: ProtocolParameters,
}
