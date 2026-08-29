use std::{collections::BTreeMap, ops::Deref};

use anyhow::{Result, bail};
use cardano_sdk::{SigningKey, Transaction, VerificationKey, transaction::state::ReadyForSigning};
use konduit_tmp::to_verifying_key;
use serde::{Deserialize, Serialize};

use crate::known_keys::KnownKeys;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyHex(#[serde(with = "hex::serde")] [u8; 32]);

impl Deref for KeyHex {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<[u8; 32]> for KeyHex {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl From<KeyHex> for SigningKey {
    fn from(value: KeyHex) -> Self {
        Self::from(value.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    keys: BTreeMap<String, KeyHex>,
}

impl Default for Config {
    fn default() -> Self {
        let keys = BTreeMap::from([("my_signing_key".to_string(), [0u8; 32].into())]);
        Self { keys }
    }
}

/// A small local keyring: `label -> signing key`
#[derive(Debug, Default, Clone)]
pub struct Keyring(BTreeMap<String, SigningKey>);

impl Keyring {
    /// `entries`: `(label, signing_key)` pairs.
    pub fn from_config(config: Config) -> Self {
        Self(
            config
                .keys
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        )
    }

    /// Direct — same `label ->` direction as `Keyring` itself, just
    /// signing key swapped for its verifying key.
    pub fn known_keys(&self) -> KnownKeys {
        KnownKeys::new(
            self.0
                .iter()
                .map(|(label, k)| (label.clone(), to_verifying_key(k.to_verification_key())))
                .collect(),
        )
    }

    pub fn sign_tx(
        &self,
        mut tx: Transaction<ReadyForSigning>,
        known_keys: &KnownKeys,
        signers: &[VerificationKey],
    ) -> Result<Transaction<ReadyForSigning>> {
        let lookup: BTreeMap<_, &SigningKey> = self
            .0
            .values()
            .map(|k| (k.to_verification_key(), k))
            .collect();
        for vkey in signers {
            let Some(signing_key) = lookup.get(vkey) else {
                let who = known_keys
                    .label_for(&to_verifying_key(*vkey))
                    .unwrap_or("<unrecognized key>");
                bail!("no signing key in keyring for {who} — required to sign this tx");
            };
            tx.sign(signing_key);
        }
        Ok(tx)
    }
}
