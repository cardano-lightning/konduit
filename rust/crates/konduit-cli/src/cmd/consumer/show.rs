use cardano_connect::CardanoConnect;
use tokio::runtime::Runtime;

use crate::config::consumer::Config;

/// Show
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config,
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
            print!("{}", config);
            return Ok(());
        } else {
            let connector = config.connector.connector()?;
            match self {
                Cmd::Address => {
                    print!("{}", config.wallet.to_address(&connector.network().into()));
                    Ok(())
                }
                Cmd::Tip { verbose } => {
                    let tip =
                        Runtime::new()?.block_on(crate::tip::Consumer::new(&connector, &config))?;
                    if verbose {
                        println!("{:#}", tip);
                    } else {
                        println!("{}", tip);
                    }
                    Ok(())
                }
                Cmd::Config => panic!("Impossible"),
            }
        }
    }
}
