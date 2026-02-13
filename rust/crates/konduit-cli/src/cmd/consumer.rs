use crate::{
    config::consumer::Config,
    env::{base::Setup, consumer::Env},
};

mod make;
mod show;
mod tx;

/// Consumer CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show an example of environment variables.
    Setup(Setup<Env>),

    /// Show info (requires env)
    #[clap(subcommand)]
    Show(show::Cmd),

    /// Make cheques and squashes
    #[clap(subcommand)]
    Make(make::Cmd),

    /// Build transactions useful to a consumer.
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self, env: Env) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            return cmd.run(env);
        }

        let config = Config::try_from(env)?;

        match self {
            Cmd::Make(cmd) => cmd.run(&config),
            Cmd::Show(cmd) => cmd.run(&config),
            Cmd::Tx(cmd) => cmd.run(&config),
            Cmd::Setup(_) => unreachable!(),
        }
    }
}
