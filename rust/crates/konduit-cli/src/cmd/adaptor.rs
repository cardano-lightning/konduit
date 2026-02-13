use crate::{config::adaptor::Config, env::adaptor::Env};

mod setup;
mod show;
mod tx;
mod verify;

/// Adaptor CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show an example of environment variables.
    Setup(setup::Cmd),

    /// Show current configuration.
    #[clap(subcommand)]
    Show(show::Cmd),

    /// Verify squashes and cheques (internally; eg not against a retainer)
    #[clap(subcommand)]
    Verify(verify::Cmd),

    /// Build transactions useful to an adaptor.
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self, env: Env) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            return cmd.run();
        }

        let config = Config::try_from(env)?;

        match self {
            Cmd::Verify(cmd) => cmd.run(&config),
            Cmd::Show(cmd) => cmd.run(&config),
            Cmd::Tx(cmd) => cmd.run(&config),
            Cmd::Setup(_) => unreachable!(),
        }
    }
}
