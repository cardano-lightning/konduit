use std::path::Path;

use clap::Subcommand;

use crate::config;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print the current config (secrets redacted)
    Show,
    /// Create an empty konduit.toml if one doesn't already exist
    Init,
    /// Set the embedded wallet signing key. Accepts a hex key, or
    /// `env:VAR_NAME` to resolve the key from an env var at read time.
    SetWallet { value: String },
    /// Clear the embedded wallet signing key
    UnsetWallet,
    /// Set the embedded add_vkey signer. Accepts a hex key, or `env:VAR_NAME`.
    SetSigner { value: String },
    /// Clear the embedded add_vkey signer
    UnsetSigner,
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if matches!(self, Cmd::Init) {
            if config_path.exists() {
                println!("{} already exists", config_path.display());
            } else {
                config::Config::default().save(config_path)?;
                println!("wrote {}", config_path.display());
            }
            return Ok(());
        }
        let mut config = config::Config::load(config_path)?;
        match self {
            Cmd::Show => {
                println!("{}", config.describe_redacted());
            }
            Cmd::SetWallet { value } => {
                config.keys_mut().set_wallet(value.to_owned());
                config.save(config_path)?;
            }
            Cmd::UnsetWallet => {
                config.keys_mut().unset_wallet();
                config.save(config_path)?;
            }
            Cmd::SetSigner { value } => {
                config.keys_mut().set_signer(value.to_owned());
                config.save(config_path)?;
            }
            Cmd::UnsetSigner => {
                config.keys_mut().unset_signer();
                config.save(config_path)?;
            }
            Cmd::Init => {
                panic!("Impossible")
            }
        }
        Ok(())
    }
}
