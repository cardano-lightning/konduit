use crate::core::{Credential, Input, Output};
use cardano_connector::CardanoConnector;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("connector error: {0}")]
    Connector(String),
}

/// Fetch utxos across a batch of (payment, optional stake) credential
/// pairs. Naive implementation: one call per pair, results merged into a
/// single `BTreeMap` (de-duplicating any utxo reachable via more than
/// one pair in the batch, and matching `utxos_at`'s own return shape).
///
/// A connector with a more efficient way to satisfy "many credential
/// pairs at once" can offer its own batched path later — this function
/// is the always-correct fallback in the meantime.
pub async fn utxo_batch<Connector: CardanoConnector>(
    connector: &Connector,
    pairs: &[(Credential, Option<Credential>)],
) -> Result<BTreeMap<Input, Output>, Error> {
    let mut utxos = BTreeMap::new();

    for (payment, stake) in pairs {
        let found = connector
            .utxos_at(payment, stake.as_ref())
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;
        utxos.extend(found);
    }

    Ok(utxos)
}
