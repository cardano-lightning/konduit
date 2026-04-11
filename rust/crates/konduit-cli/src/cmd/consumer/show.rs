use crate::config::consumer::Config;
use konduit_data::{Keytag, Tag};
use tokio::runtime::Runtime;

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

        match self {
            Cmd::Address => {
                let network_id = config
                    .connector
                    .network_id()
                    .expect("connector network should always be available from config");
                print!(
                    "{}",
                    config.wallet.to_verification_key().to_address(network_id)
                );
                Ok(())
            }
            Cmd::Keytag { tag } => {
                print!("{}", Keytag::new(config.wallet.to_verification_key(), tag));
                Ok(())
            }
            Cmd::Tip { verbose } => {
                let connector = config.connector.connector()?;
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
