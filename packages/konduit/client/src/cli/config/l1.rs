use std::path::Path;

use cardano_sdk::{Address, address::kind};
use clap::{Subcommand, ValueEnum};
use konduit_data::Duration;

use crate::config;
use crate::core::Credential;
use crate::l1::{BoundsPolicy, SubmitPolicy};

use super::with_config;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SubmitPolicyArg {
    ViaConnector,
    ViaWallet,
}

impl From<SubmitPolicyArg> for SubmitPolicy {
    fn from(arg: SubmitPolicyArg) -> Self {
        match arg {
            SubmitPolicyArg::ViaConnector => SubmitPolicy::ViaConnector,
            SubmitPolicyArg::ViaWallet => SubmitPolicy::ViaWallet,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print the current l1 config
    Show,
    /// Set how transactions get submitted
    SetSubmitPolicy { policy: SubmitPolicyArg },
    /// Set the transaction validity window, in seconds from build time
    SetBoundsWindow { window: Duration },
    /// Enable or disable autocomplete
    SetAutocomplete { value: bool },
    /// Set the reference script address (bech32)
    SetReferenceScriptAddress { address: Address<kind::Shelley> },
    /// Set the preferred change address (bech32), if different from wallet
    SetChangeAddress { address: Address<kind::Any> },
    /// Add a delegation credential
    AddDelegation { credential: Credential },
    /// Remove a delegation credential
    RemoveDelegation { credential: Credential },
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if let Cmd::Show = self {
            let cfg = config::Config::load(config_path)?;
            println!("{:?}", cfg.l1());
            return Ok(());
        }

        with_config(config_path, |cfg| {
            match self {
                Cmd::SetSubmitPolicy { policy } => cfg.l1_mut().set_submit_policy((*policy).into()),
                Cmd::SetBoundsWindow { window } => cfg
                    .l1_mut()
                    .set_bounds_policy(BoundsPolicy::new(window.to_owned())),
                Cmd::SetAutocomplete { value } => cfg.l1_mut().set_autocomplete(*value),
                Cmd::SetReferenceScriptAddress { address } => {
                    cfg.l1_mut().set_reference_script_address(address.clone())
                }
                Cmd::SetChangeAddress { address } => {
                    cfg.l1_mut().set_change_address(address.clone())
                }
                Cmd::AddDelegation { credential } => {
                    cfg.l1_mut().add_delegation(credential.clone())
                }
                Cmd::RemoveDelegation { credential } => cfg.l1_mut().remove_delegation(credential),
                Cmd::Show => unreachable!("handled above"),
            }
            Ok(())
        })
    }
}
