use cardano_sdk::{NetworkId, ProtocolParameters};

#[derive(Debug, Clone)]
pub struct NetworkParameters {
    pub network_id: NetworkId,
    pub protocol_parameters: ProtocolParameters,
}
