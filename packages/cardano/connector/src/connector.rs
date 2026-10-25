use cardano_sdk::{
    Credential, Input, Network, Output, PosixMilliseconds, ProtocolParameters, Slot, Transaction,
    transaction::state,
};
use std::collections::BTreeMap;

pub trait CardanoConnector {
    fn network(&self) -> Network;

    fn health(&self) -> impl Future<Output = anyhow::Result<String>>;

    fn protocol_parameters(&self) -> impl Future<Output = anyhow::Result<ProtocolParameters>>;

    /// The chain's current tip — the slot the chain is at right now.
    ///
    /// Together with the protocol parameters (which carry the chain's slot
    /// length and time anchor) this is what lets us express a validity range
    /// in the *chain's* clock rather than in the wallet's wall clock.
    fn tip_slot(&self) -> impl Future<Output = anyhow::Result<Slot>>;

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

/// The chain's current time, in POSIX milliseconds.
///
/// This is the chain's clock, not the wallet's wall clock: the tip slot is
/// converted with the chain's own slot length and time anchor (both carried
/// by the protocol parameters). Anchoring a transaction's validity range (or a
/// cheque's timeout) here keeps them comparable with what the ledger hands to
/// a Plutus script.
///
/// Connectors that cannot report a tip (our own HTTP API does not expose one)
/// fall back to the wallet's wall clock, which is what the public networks
/// use anyway.
pub async fn chain_now(connector: &impl CardanoConnector) -> anyhow::Result<PosixMilliseconds> {
    let params = connector.protocol_parameters().await?;
    match connector.tip_slot().await {
        Ok(tip) => Ok(params.slot_to_posix(tip)),
        Err(_) => Ok(wall_clock_now()),
    }
}

/// The wallet's wall clock, in POSIX milliseconds.
pub fn wall_clock_now() -> PosixMilliseconds {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    PosixMilliseconds::new(millis)
}
