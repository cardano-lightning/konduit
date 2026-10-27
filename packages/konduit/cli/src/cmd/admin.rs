use crate::{config::admin::Config, env::admin::Env, shared::Setup};

mod show;
mod tx;

/// Admin CLI
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Create a configuration with sensible defaults.
    ///
    /// Defaults can be overridden manually via options or via environment variables.
    /// See also admin --help.
    Setup,

    /// Show current configuration.
    #[clap(subcommand)]
    Show(show::Cmd),

    /// Build transactions related to admin duties.
    #[clap(subcommand)]
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) async fn run(self, env: Env) -> anyhow::Result<()> {
        match self {
            Cmd::Show(cmd) => {
                let config = Config::try_from(env)?;
                cmd.run(&config).await
            }
            Cmd::Tx(cmd) => {
                let config = Config::try_from(env)?;
                cmd.run(&config).await
            }
            Cmd::Setup => env.setup(),
        }
    }
}
