use crate::config::admin::Config;

/// Show
#[derive(Debug, clap::Subcommand)]
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
    pub(crate) async fn run(self, config: &Config) -> anyhow::Result<()> {
        if let Cmd::Config = self {
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
            Cmd::Tip { verbose } => {
                let connector = config.connector.connector().await?;
                let tip = crate::tip::Admin::new(&connector, config).await?;
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
