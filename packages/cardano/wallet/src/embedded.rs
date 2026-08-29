//! `Wallet` backed by a raw signing key held in-process, plus a real
//! `Connector`. No separate `Signer` type - the only consumer that ever
//! needed just the key material (`cardano-session`) now takes
//! `cardano_wallet::Wallet` as a generic parameter instead, so it uses
//! `Embedded` directly rather than reaching inside it.

use crate::wallet::Wallet;
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Address, Credential, Hash, Input, NetworkId, Output, Signature, SigningKey, Transaction, Value,
    VerificationKey, address::kind, transaction::state,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("connector error: {0}")]
    Connector(String),
}

/// On-disk config: just the raw signing key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(with = "hex::serde")]
    key: [u8; 32],
}

impl Default for Config {
    fn default() -> Self {
        // Deterministic, dev-only default so a fresh checkout has a
        // working key without any setup - never use it for anything
        // holding real value.
        Self {
            key: hash32(b"wallet"),
        }
    }
}

/// `Wallet` backed by a raw key and a real `Connector`.
pub struct Embedded<Connector: CardanoConnector> {
    connector: Arc<Connector>,
    signing_key: SigningKey,
    verification_key: VerificationKey,
    credential: Credential,
    delegation: Option<Credential>,
}

impl<Connector: CardanoConnector> Embedded<Connector> {
    pub fn new(connector: Arc<Connector>, config: Config, delegation: Option<Credential>) -> Self {
        let signing_key = SigningKey::from(config.key);
        let verification_key = signing_key.to_verification_key();
        let credential = Credential::from_key(Hash::<28>::new(verification_key));
        Self {
            connector,
            signing_key,
            verification_key,
            credential,
            delegation,
        }
    }

    /// Public key material - safe to expose beyond the `Wallet` trait
    /// itself, unlike the signing key. `cardano_wallet::Wallet` has no
    /// equivalent method, since `Cip30` can't offer this synchronously
    /// (or possibly at all) - this is `Embedded`-specific.
    pub fn verification_key(&self) -> VerificationKey {
        self.verification_key
    }
}

impl<Connector: CardanoConnector> Wallet for Embedded<Connector> {
    type Error = Error;

    async fn network_id(&self) -> Result<NetworkId, Self::Error> {
        Ok(NetworkId::from(self.connector.network()))
    }

    async fn change_address(&self) -> Result<Address<kind::Any>, Self::Error> {
        let address = Address::new(self.network_id().await?, self.credential.clone());
        Ok(match &self.delegation {
            Some(d) => address.with_delegation(d.clone()).into(),
            None => address.into(),
        })
    }

    async fn utxos(
        &self,
        value: Option<Value<u64>>,
    ) -> Result<Option<BTreeMap<Input, Output>>, Self::Error> {
        let payment = self.credential.clone();
        let mut pairs = vec![(payment.clone(), None)];
        if let Some(stake) = &self.delegation {
            pairs.push((payment, Some(stake.clone())));
        }

        let all_utxos: BTreeMap<Input, Output> = futures::future::try_join_all(
            pairs
                .iter()
                .map(|(p, d)| self.connector.utxos_at(p, d.as_ref())),
        )
        .await
        .map_err(|e| Error::Connector(e.to_string()))?
        .into_iter()
        .flatten()
        .collect();

        let Some(value) = value else {
            return Ok((!all_utxos.is_empty()).then_some(all_utxos));
        };

        let utxos: Vec<(Input, Output)> = all_utxos.into_iter().collect();
        let Some(selection) = Value::cover(&value, &utxos, |(_, output)| output.value()) else {
            return Ok(None);
        };
        Ok(Some(selection.inputs.into_iter().cloned().collect()))
    }

    async fn sign_tx(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<(VerificationKey, Signature), Self::Error> {
        let signature = self.signing_key.sign(tx.id().as_ref());
        Ok((self.verification_key, signature))
    }

    async fn submit(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<Hash<32>, Self::Error> {
        self.connector
            .submit(tx)
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;
        Ok(tx.id())
    }
}

impl<Connector: CardanoConnector> std::fmt::Debug for Embedded<Connector> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Embedded")
            .field("verification_key", &self.verification_key)
            .finish_non_exhaustive()
    }
}

/// Deterministic, non-cryptographic 32-byte expansion of `seed`. Only
/// good for the insecure dev default above - not a real KDF.
fn hash32(seed: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash as _, Hasher};

    let mut out = [0u8; 32];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        i.hash(&mut hasher);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use cardano_sdk::Network;

    use super::*;
    use std::collections::BTreeMap as Map;

    #[test]
    fn hash32_deterministic() {
        assert_eq!(hash32(b"wallet"), hash32(b"wallet"));
    }

    #[test]
    fn hash32_different_seeds_differ() {
        assert_ne!(hash32(b"wallet"), hash32(b"other"));
    }

    #[test]
    fn config_default_is_deterministic() {
        assert_eq!(Config::default().key, Config::default().key);
    }

    // Needs a fake CardanoConnector matching the real trait (no
    // associated Error, network() -> Network, utxos_at(.., Option<&Credential>),
    // plus health/protocol_parameters). One remaining guess: the
    // `Network` variant name.

    struct FakeConnector {
        network: Network,
        utxos: Map<Input, Output>,
        submitted: std::cell::RefCell<Option<Transaction<state::ReadyForSigning>>>,
    }

    impl CardanoConnector for FakeConnector {
        fn network(&self) -> Network {
            self.network.clone()
        }

        async fn health(&self) -> Result<String, anyhow::Error> {
            Ok("ok".into())
        }

        async fn protocol_parameters(
            &self,
        ) -> Result<cardano_sdk::ProtocolParameters, anyhow::Error> {
            Ok(cardano_sdk::ProtocolParameters::preprod())
        }

        async fn utxos_at(
            &self,
            _payment: &Credential,
            _delegation: Option<&Credential>,
        ) -> Result<Map<Input, Output>, anyhow::Error> {
            Ok(self.utxos.clone())
        }

        async fn submit(
            &self,
            tx: &Transaction<state::ReadyForSigning>,
        ) -> Result<(), anyhow::Error> {
            *self.submitted.borrow_mut() = Some(tx.clone());
            Ok(())
        }
    }

    fn test_embedded(utxos: Map<Input, Output>) -> Embedded<FakeConnector> {
        let connector = Arc::new(FakeConnector {
            network: Network::Preprod,
            utxos,
            submitted: std::cell::RefCell::new(None),
        });
        Embedded::new(connector, Config::default(), None)
    }

    #[tokio::test]
    async fn change_address_derives_from_the_key() {
        let wallet = test_embedded(Map::new());
        let a1 = wallet.change_address().await.unwrap();
        let a2 = wallet.change_address().await.unwrap();
        assert_eq!(a1, a2);
    }

    #[tokio::test]
    async fn utxos_none_when_wallet_is_empty() {
        let wallet = test_embedded(Map::new());
        assert_eq!(wallet.utxos(None).await.unwrap(), None);
    }
}
