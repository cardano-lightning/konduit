use crate::time;

/// FIXME :: Use #[from].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("time: {0}")]
    Time(#[from] time::Error),
    #[error("nothing to do: no channels to open and no konduit utxos found")]
    NothingToDo,
    #[error("no reference script address. Must be set to find reference script")]
    NoReferenceScriptAddress,
    #[error("no reference script utxo cached: call pull_reference_script first")]
    NoReferenceScript,
    #[error("no network parameters cached: call pull_network_parameters first")]
    NoNetworkParameters,
    #[error("no change address set in config: build a Config with one set and use it")]
    NoChangeAddress,
    #[error("no tx cached: call build first")]
    NoStatedTx,
    #[error("connector error: {0}")]
    Connector(String),
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("failed to build transaction: {0}")]
    Tx(String),
    #[error("signing error: {0}")]
    Signing(String),
}
