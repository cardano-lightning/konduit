use konduit_data::{Keytag, Locked, Squash};

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
    },
    /// Verify a locked cheque
    Locked {
        /// Channel keytag
        #[arg(long)]
        keytag: Keytag,
        /// Will parse either <cheque_body,signature> of the cbor of `Cheque::Locked`
        #[arg(long, value_parser = parse_locked )]
        locked: Locked,
    },
}
impl Cmd {
    pub(crate) fn run(self, _config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Squash { keytag, squash } => {
                let (key, tag) = keytag.split();
                println!("{}", squash.verify(&key, &tag));
                Ok(())
            }
            Cmd::Locked { keytag, locked } => {
                let (key, tag) = keytag.split();
                println!("{}", locked.verify(&key, &tag));
                Ok(())
            }
        }
    }
}
