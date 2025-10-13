use std::collections::BTreeMap;

use anyhow::Result;

use cardano_tx_builder::{Credential, Input, NetworkId, Output, ProtocolParameters};

pub trait CardanoConnect {
    fn network(&self) -> NetworkId;
    fn health(&self) -> impl std::future::Future<Output = Result<String>> + Send;
    fn protocol_parameters(
        &self,
    ) -> impl std::future::Future<Output = Result<ProtocolParameters>> + Send;
    fn utxos_at(
        &self,
        payment: &Credential,
        delegation: &Option<Credential>,
    ) -> impl std::future::Future<Output = Result<BTreeMap<Input, Output>>> + Send;
    fn submit(&self, tx: Vec<u8>) -> impl std::future::Future<Output = Result<String>> + Send;
}
