use crate::network::Network;
use cardano_tx_builder::{
    Credential, Input, Output, ProtocolParameters, Transaction, transaction::state,
};
use std::collections::BTreeMap;

pub trait CardanoConnect {
    fn network(&self) -> Network;

    fn health(&self) -> impl Future<Output = anyhow::Result<String>>;

    fn protocol_parameters(&self) -> impl Future<Output = anyhow::Result<ProtocolParameters>>;

    /// If delegation is None then it _should_ be ignored:
    /// Any address with matching payment credential should be returned.
    fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> impl Future<Output = anyhow::Result<BTreeMap<Input, Output>>>;

    fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = anyhow::Result<()>>;
}
