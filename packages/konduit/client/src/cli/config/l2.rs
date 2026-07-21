use std::path::Path;

use cardano_sdk::Input;
use clap::Subcommand;
use konduit_data::{Duration, Tag};
use konduit_wire::reg::cobbl3::Credential;

use crate::config;
use crate::l2::{RegPolicy, SquashPolicy};
use crate::{server, server::codec};

use super::with_config;

#[derive(Debug, Clone, Subcommand)]
pub enum RegPolicyArg {
    /// No registration auth required
    None,
    /// Require auth, valid for the given window
    Auth { seconds: u64 },
}

impl From<RegPolicyArg> for RegPolicy {
    fn from(arg: RegPolicyArg) -> Self {
        match arg {
            RegPolicyArg::None => RegPolicy::None,
            RegPolicyArg::Auth { seconds } => RegPolicy::Auth(Duration::from_secs(seconds)),
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum SquashPolicyArg {
    /// No automatic squash handling
    Manual,
    /// Automatically retry, tolerating any expiry
    Lenient { retry: u8 },
    /// Automatically retry, rejecting proposals older than last received
    RejectOld { retry: u8 },
}

impl From<SquashPolicyArg> for SquashPolicy {
    fn from(arg: SquashPolicyArg) -> Self {
        match arg {
            SquashPolicyArg::Manual => SquashPolicy::Manual,
            SquashPolicyArg::Lenient { retry } => SquashPolicy::lenient(retry),
            SquashPolicyArg::RejectOld { retry } => SquashPolicy::reject_old(retry),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print the config for a given l2 tag
    Show { tag: Tag },
    /// Add a new l2 entry for a tag, pointing at a server
    AddL2 {
        tag: Tag,
        base_url: String,
        #[arg(long, default_value = "cbor")]
        codec: codec::Kind,
    },
    /// Remove an l2 entry
    RemoveL2 { tag: Tag },
    /// Set the registration policy for a given l2 tag
    SetRegPolicy {
        tag: Tag,
        #[command(subcommand)]
        policy: RegPolicyArg,
    },
    /// Set the squash policy for a given l2 tag
    SetSquashPolicy {
        tag: Tag,
        #[command(subcommand)]
        policy: SquashPolicyArg,
    },
    /// Set the focused input for a given l2 tag
    SetFocus { tag: Tag, input: Input },
    /// Clear the focused input for a given l2 tag
    UnsetFocus { tag: Tag },
    /// Set the credential for a given l2 tag
    SetCredential { tag: Tag, credential: Credential },
    /// Clear the credential for a given l2 tag
    UnsetCredential { tag: Tag },
}

impl Cmd {
    pub fn run(&self, config_path: &Path) -> anyhow::Result<()> {
        if let Cmd::Show { tag } = self {
            let cfg = config::Config::load(config_path)?;
            match cfg.l2(tag) {
                Some(l2) => println!("{:?}", l2),
                None => println!("no l2 config for tag {tag:?}"),
            }
            return Ok(());
        }

        with_config(config_path, |cfg| match self {
            Cmd::AddL2 {
                tag,
                base_url,
                codec,
            } => {
                cfg.add_l2(tag.clone(), server::Config::new(base_url.clone(), *codec));
                Ok(())
            }
            Cmd::RemoveL2 { tag } => {
                if !cfg.remove_l2(tag) {
                    println!("no l2 config for tag {tag:?}");
                }
                Ok(())
            }
            Cmd::SetRegPolicy { tag, policy } => {
                get_l2_mut(cfg, tag)?.set_reg_policy(policy.clone().into());
                Ok(())
            }
            Cmd::SetSquashPolicy { tag, policy } => {
                get_l2_mut(cfg, tag)?.set_squash_policy(policy.clone().into());
                Ok(())
            }
            Cmd::SetFocus { tag, input } => {
                get_l2_mut(cfg, tag)?.set_focus(Some(input.clone()));
                Ok(())
            }
            Cmd::UnsetFocus { tag } => {
                get_l2_mut(cfg, tag)?.set_focus(None);
                Ok(())
            }
            Cmd::SetCredential { tag, credential } => {
                get_l2_mut(cfg, tag)?.set_credential(Some(credential.clone()));
                Ok(())
            }
            Cmd::UnsetCredential { tag } => {
                get_l2_mut(cfg, tag)?.set_credential(None);
                Ok(())
            }
            Cmd::Show { .. } => unreachable!("handled above"),
        })
    }
}

fn get_l2_mut<'a>(
    config: &'a mut config::Config,
    tag: &Tag,
) -> anyhow::Result<&'a mut crate::l2::Config> {
    config
        .l2_mut(tag)
        .ok_or_else(|| anyhow::anyhow!("no l2 config for tag {tag:?} — add one first"))
}
