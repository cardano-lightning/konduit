use crate::config::adaptor::Config;
use tokio::runtime::Runtime;

/// Show
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config,
    /// Show adaptor constants used by consumer.
    Constants,
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
        }

        if let Cmd::Constants = self {
            print!(
                "{},{}",
                config.wallet.to_verification_key(),
                config.close_period
            );
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
            Cmd::Tip { verbose } => {
                let connector = config.connector.connector()?;
                let tip = Runtime::new()?.block_on(crate::tip::Adaptor::new(&connector, config))?;
                if verbose {
                    println!("{:#}", tip);
                } else {
                    println!("{}", tip);
                }
                Ok(())
            }
            Cmd::Config | Cmd::Constants => unreachable!(),
        }
    }
}
