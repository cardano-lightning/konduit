use crate::network::Network;
use cardano_tx_builder::{
    Credential, Input, Output, ProtocolParameters, Transaction, transaction::state,
};
use std::collections::BTreeMap;
use trait_variant::make;

#[make(CardanoConnectDyn: Send)]
pub trait CardanoConnect {
    fn network(&self) -> Network;

    async fn health(&self) -> anyhow::Result<String>;

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters>;

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>>;

    async fn submit(&self, transaction: &Transaction<state::ReadyForSigning>)
    -> anyhow::Result<()>;
}
