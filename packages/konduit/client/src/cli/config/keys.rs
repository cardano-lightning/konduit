use std::path::Path;

use clap::Subcommand;

use super::with_config;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Set the embedded wallet signing key. Accepts a hex key, or
    /// `env:VAR_NAME` to resolve the key from an env var at read time.
    /// Use `<cli> keygen` for random bytes.
    SetWallet { value: String },
    /// Clear the embedded wallet signing key
    UnsetWallet,
    /// Set the embedded add_vkey signer. Accepts a hex key, or `env:VAR_NAME`.
    /// Use `<cli> keygen` for random bytes.
    SetSigner { value: String },
    /// Clear the embedded add_vkey signer
    UnsetSigner,
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        with_config(config_path, |cfg| {
            match self {
                Cmd::SetWallet { value } => cfg.keys_mut().set_wallet(value.to_owned()),
                Cmd::UnsetWallet => cfg.keys_mut().unset_wallet(),
                Cmd::SetSigner { value } => cfg.keys_mut().set_signer(value.to_owned()),
                Cmd::UnsetSigner => cfg.keys_mut().unset_signer(),
            }
            Ok(())
        })
    }
}
