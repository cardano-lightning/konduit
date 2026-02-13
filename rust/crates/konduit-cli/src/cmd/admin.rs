use crate::{
    config::admin::Config,
    env::{admin::Env, base::Setup},
};

mod show;
mod tx;

/// Admin CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show an example of environment variables.
    Setup(Setup<Env>),

    /// Show current configuration.
    #[clap(subcommand)]
    Show(show::Cmd),

    /// Build transactions related to admin duties.
    #[clap(subcommand)]
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self, env: Env) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            return cmd.run(env);
        }

        let config = Config::try_from(env)?;

        match self {
            Cmd::Show(cmd) => cmd.run(&config),
            Cmd::Tx(cmd) => cmd.run(&config),
            Cmd::Setup(_) => unreachable!(),
        }
    }
}
