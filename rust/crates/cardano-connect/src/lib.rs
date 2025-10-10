use anyhow::Result;

use cardano_tx_builder::{Credential, NetworkId, ProtocolParameters, ResolvedInput};

pub trait CardanoConnect {
    fn network(&self) -> NetworkId;
    fn health(&self) -> impl std::future::Future<Output = Result<String>> + Send;
    fn protocol_parameters(&self) -> impl std::future::Future<Output = ProtocolParameters> + Send;
    fn resolved_inputs_at(
        &self,
        payment: &Credential,
        delegation: &Option<Credential>,
    ) -> impl std::future::Future<Output = Result<Vec<ResolvedInput>>> + Send;
    fn submit(&self, tx: Vec<u8>) -> impl std::future::Future<Output = Result<String>> + Send;
}
