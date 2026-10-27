use konduit_data::{Locked, Squash};
use konduit_tmp::Keytag;
use konduit_tx::to_verifying_key;
use serde::{Deserialize, Serialize};

use crate::output::{self, OutputJson};
use crate::{
    cmd::parsers::{parse_locked, parse_squash},
    config::adaptor::Config,
};

/// Verify
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Verify a squash
    Squash {
        /// Channel keytag
        #[arg(long)]
        keytag: Keytag,
        /// Squash either (hex) cbor or CSV of body-as-cbor and signature
        #[arg(long, value_parser = parse_squash )]
        squash: Squash,
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Verify a locked cheque
    Locked {
        /// Channel keytag
        #[arg(long)]
        keytag: Keytag,
        /// Will parse either <cheque_body,signature> of the cbor of `Cheque::Locked`
        #[arg(long, value_parser = parse_locked )]
        locked: Locked,
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
}

impl Cmd {
    pub(crate) fn run(self, _config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Squash {
                keytag,
                squash,
                output_json,
            } => {
                let (key, tag) = keytag.split();
                let valid = squash.try_verify(&to_verifying_key(key), &tag).is_ok();
                if output_json {
                    let dto = VerifyResult { valid };
                    output::render(OutputJson(true), &dto)?;
                } else {
                    println!("{}", valid);
                }
                Ok(())
            }
            Cmd::Locked {
                keytag,
                locked,
                output_json,
            } => {
                let (key, tag) = keytag.split();
                let valid = locked.try_verify(&to_verifying_key(key), &tag).is_ok();
                if output_json {
                    let dto = VerifyResult { valid };
                    output::render(OutputJson(true), &dto)?;
                } else {
                    println!("{}", valid);
                }
                Ok(())
            }
        }
    }
}
