use std::path::Path;

use clap::Subcommand;

pub mod keys;
pub mod l1;
pub mod l2;
pub mod server;

use crate::config;

/// Load the config at `path`, apply `f`, then persist. Centralizes the
/// load -> mutate -> save cycle every mutating subcommand needs.
pub(crate) fn with_config(
    path: &Path,
    f: impl FnOnce(&mut config::Config) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut cfg = config::Config::load(path)?;
    f(&mut cfg)?;
    cfg.save(path)
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Create an empty konduit.toml if one doesn't already exist
    Init,
    /// Print the current config (secrets redacted)
    Show,
    /// Get and set wallet/signer keys
    #[clap(subcommand)]
    Keys(keys::Cmd),
    /// Get and set L1 policy config
    #[clap(subcommand)]
    L1(l1::Cmd),
    /// Get and set L2 policy config
    #[clap(subcommand)]
    L2(l2::Cmd),
    /// Get and set known servers
    #[clap(subcommand)]
    Server(server::Cmd),
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        match self {
            Cmd::Init => {
                if config_path.exists() {
                    println!("{} already exists", config_path.display());
                } else {
                    config::Config::default().save(config_path)?;
                    println!("wrote {}", config_path.display());
                }
                Ok(())
            }
            Cmd::Keys(cmd) => cmd.run(config_path),
            Cmd::L1(cmd) => cmd.run(config_path),
            Cmd::L2(cmd) => cmd.run(config_path),
            Cmd::Server(cmd) => cmd.run(config_path),
            Cmd::Show => {
                let cfg = config::Config::load(config_path)?;
                println!("{}", cfg.describe_redacted());
                Ok(())
            }
        }
    }
}
