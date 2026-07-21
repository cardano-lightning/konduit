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

    /// Redacted summary for `config show` — never prints key material,
    /// but does say whether each key comes from a literal value or an env ref.
    pub fn describe_redacted(&self) -> String {
        format!(
            "wallet: {}\nsigner: {}",
            Self::describe_one(self.wallet.as_deref()),
            Self::describe_one(self.signer.as_deref()),
        )
    }

    fn describe_one(raw: Option<&str>) -> String {
        match raw {
            None => "unset".to_string(),
            Some(v) if v.starts_with("env:") => format!("set (via {v})"),
            Some(_) => "set (literal)".to_string(),
        }
    }
}
