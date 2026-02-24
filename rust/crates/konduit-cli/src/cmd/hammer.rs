use crate::{config::hammer::Config, env::hammer::Env, shared::Setup};

mod make;
mod show;
mod tx;

/// Hammer CLI
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Create a configuration with sensible defaults.
    ///
    /// Defaults can be overridden manually via options or via environment variables.
    /// See also hammer --help.
    Setup,

    /// Show info (requires env)
    #[clap(subcommand)]
    Show(show::Cmd),

    /// Make cheques and squashes
    #[clap(subcommand)]
    Make(make::Cmd),

    // /// Build transactions useful to a hammer.
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self, env: Env) -> anyhow::Result<()> {
        if let Cmd::Setup = self {
            return env.setup();
        }

        let config = Config::try_from(env)?;

        match self {
            Cmd::Make(cmd) => cmd.run(&config),
            Cmd::Show(cmd) => cmd.run(&config),
            Cmd::Tx(cmd) => cmd.run(&config),
            Cmd::Setup => unreachable!(),
        }
    }
}
