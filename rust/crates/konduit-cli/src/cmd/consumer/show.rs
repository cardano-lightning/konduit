use cardano_connect::CardanoConnect;
use konduit_data::{Keytag, Tag};
use tokio::runtime::Runtime;

use crate::config::consumer::Config;

/// Show
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config,
    /// Show Keytag
    Keytag { tag: Tag },
    /// Show address.
    Address,
    /// Show tip
    Tip {
        // Verbosity
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
}

impl Cmd {
    pub(crate) fn run(self, config: &Config) -> anyhow::Result<()> {
        if let Cmd::Config = self {
            // Separated out since connector might not be setup correctly.
            print!("{}", config);
            return Ok(());
        }

        let connector = config.connector.connector()?;
        match self {
            Cmd::Address => {
                print!(
                    "{}",
                    config
                        .wallet
                        .to_verification_key()
                        .to_address(connector.network().into())
                );
                Ok(())
            }
            Cmd::Keytag { tag } => {
                print!("{}", Keytag::new(config.wallet.to_verification_key(), tag));
                Ok(())
            }
            Cmd::Tip { verbose } => {
                let tip =
                    Runtime::new()?.block_on(crate::tip::Consumer::new(&connector, config))?;
                if verbose {
                    println!("{:#}", tip);
                } else {
                    println!("{}", tip);
                }
                Ok(())
            }
            Cmd::Config => unreachable!(),
        }
    }
}
