//! # Keys
//!
//! Optional Embedded wallet and add_vkey signing key material.
//!
//! Each key is stored as a raw string, either a hex-encoded key or an
//! `env:VAR_NAME` reference (see `crate::env::resolve`). This lets
//! secrets be kept out of `konduit.toml` entirely — the file just
//! points at whichever env var the user chooses to populate at
//! runtime — while still supporting a plain hex value directly in
//! the file for local dev.

use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Set if there is an embedded wallet. Either a hex key or `env:VAR_NAME`.
    #[n(0)]
    wallet: Option<String>,
    /// Set if there is an embedded signer `add_vkey`. Either a hex key or `env:VAR_NAME`.
    #[n(1)]
    signer: Option<String>,
}

impl Config {
    pub fn set_wallet(&mut self, value: String) {
        self.wallet = Some(value);
    }

    pub fn unset_wallet(&mut self) {
        self.wallet = None;
    }

    pub fn set_signer(&mut self, value: String) {
        self.signer = Some(value);
    }

    pub fn unset_signer(&mut self) {
        self.signer = None;
    }
}

#[cfg(feature = "cli")]
use cardano_sdk::{Credential, Hash, NetworkId, SigningKey};

#[cfg(feature = "cli")]
impl Config {
    /// Effective wallet key, resolving `env:VAR_NAME` if that's what's stored.
    pub fn wallet(&self) -> anyhow::Result<Option<[u8; 32]>> {
        Self::resolve(self.wallet.as_deref())
    }

    /// Effective signer key, resolving `env:VAR_NAME` if that's what's stored.
    pub fn signer(&self) -> anyhow::Result<Option<[u8; 32]>> {
        Self::resolve(self.signer.as_deref())
    }

    fn resolve(raw: Option<&str>) -> anyhow::Result<Option<[u8; 32]>> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let hex_str = crate::cli::env::resolve(raw)?;
        let bytes = hex::decode(hex_str.trim())?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("key must be 32 bytes hex-encoded"))?;
        Ok(Some(arr))
    }

    /// Redacted summary for `config show` — never prints raw key bytes,
    /// but derives and displays the verifying key (and, for the wallet,
    /// its hash and per-network addresses) so the operator can confirm
    /// which identity is configured without exposing the signing key.
    pub fn show(&self) -> String {
        format!(
            "wallet:\n{}\nsigner:\n{}",
            self.show_wallet(),
            self.show_signer(),
        )
    }

    fn show_signer(&self) -> String {
        let Some(raw) = self.signer.as_deref() else {
            return "  unset".to_string();
        };
        match self.signer() {
            Ok(Some(bytes)) => {
                let vk = SigningKey::from(bytes.clone()).to_verification_key();
                format!(
                    "  verifying_key: {vk}\n  source: {}",
                    Self::source_label(raw)
                )
            }
            Ok(None) => "  unset".to_string(),
            Err(e) => format!("  error resolving signer: {e}"),
        }
    }

    fn show_wallet(&self) -> String {
        let Some(raw) = self.wallet.as_deref() else {
            return "  unset".to_string();
        };
        match self.wallet() {
            Ok(Some(bytes)) => {
                let vk = SigningKey::from(bytes.clone()).to_verification_key();
                let vk_hash = Hash::<28>::new(vk);
                let mainnet_address =
                    cardano_sdk::Address::new(NetworkId::MAINNET, Credential::from_key(vk_hash));
                let testnet_address =
                    cardano_sdk::Address::new(NetworkId::TESTNET, Credential::from_key(vk_hash));
                format!(
                    "  verifying_key: {vk}\n  verifying_key_hash: {vk_hash}\n  mainnet: {mainnet_address}\n  testnet: {testnet_address}\n  source: {}",
                    Self::source_label(raw)
                )
            }
            Ok(None) => "  unset".to_string(),
            Err(e) => format!("  error resolving wallet: {e}"),
        }
    }

    fn source_label(raw: &str) -> &str {
        if raw.starts_with("env:") {
            "env"
        } else {
            "literal"
        }
    }
}
