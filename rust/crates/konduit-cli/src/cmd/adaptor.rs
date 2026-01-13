use crate::env::adaptor::Env;

mod setup;
mod show;
mod tx;
mod verify;

/// Adaptor CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Setup env.
    Setup(setup::Cmd),
    /// Show info (requires env)
    #[clap(subcommand)]
    Show(show::Cmd),
    /// Verify squashes and cheques (internally eg not against a retainer)
    #[clap(subcommand)]
    Verify(verify::Cmd),
    /// Txs
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            cmd.run()
        } else {
            let e = Env::load()?;
            let config = e.to_config()?;
            match self {
                Cmd::Verify(cmd) => cmd.run(&config),
                Cmd::Show(cmd) => cmd.run(&config),
                Cmd::Tx(cmd) => cmd.run(&config),
                Cmd::Setup(_) => panic!("Impossible"),
            }
        }
    }
}
