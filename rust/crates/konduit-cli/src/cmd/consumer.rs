use crate::env::consumer::Env;

mod make;
mod setup;
mod show;
mod tx;

/// Consumer CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Make cheques and squashes
    #[clap(subcommand)]
    Make(make::Cmd),
    /// Setup env.
    Setup(setup::Cmd),
    /// Show info (requires env)
    #[clap(subcommand)]
    Show(show::Cmd),
    /// Txs
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            cmd.run()
        } else {
            let config = Env::load()?.to_config()?;
            match self {
                Cmd::Make(cmd) => cmd.run(&config),
                Cmd::Show(cmd) => cmd.run(&config),
                Cmd::Tx(cmd) => cmd.run(&config),
                Cmd::Setup(_) => panic!("oops"),
            }
        }
    }
}
