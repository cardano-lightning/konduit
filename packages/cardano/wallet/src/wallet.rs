//! `Wallet`: the surface every component in this crate depends on -
//! address, UTXOs, signing, submission - regardless of whether keys live
//! in-process (`Embedded`) or in a browser extension (`Cip30`, wasm32
//! only). Backend choice is a build-time decision, so this stays a plain
//! `trait` with `impl Future` returns rather than paying for `dyn`-safety
//! nobody needs.

use cardano_sdk::{
    Address, Hash, Input, NetworkId, Output, Signature, Transaction, Value, VerificationKey,
    address::kind, transaction::state,
};
use std::{collections::BTreeMap, future::Future};

/// Based on CIP-30. At least permits a CIP-30 wallet to be wrapped and impl this trait.
pub trait Wallet {
    type Error: std::error::Error + Send + Sync + 'static;

    fn network_id(&self) -> impl Future<Output = Result<NetworkId, Self::Error>>;

    /// `api.getChangeAddress()`. The wallet's preferred address for receiving change
    fn change_address(&self) -> impl Future<Output = Result<Address<kind::Any>, Self::Error>>;

    /// `api.getUtxos(value, paginate)`. Pagination is not honored.
    /// `value`, when given, IS honored and returns `None` if the wallet cannot satisfy value
    fn utxos(
        &self,
        value: Option<Value<u64>>,
    ) -> impl Future<Output = Result<Option<BTreeMap<Input, Output>>, Self::Error>>;

    /// `api.signTx(tx, partialSign: true)`. Always requests a partial signature
    fn sign_tx(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = Result<(VerificationKey, Signature), Self::Error>>;

    /// Analogue to `api.submitTx(tx: cbor<transaction>): Promise<hash32>`.
    fn submit(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = Result<Hash<32>, Self::Error>>;
}
