use crate::env::adaptor::Env;

mod setup;
mod show;
// mod tx;

/// Adaptor CLI
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Setup env.
    Setup(setup::Cmd),
    /// Show info (requires env)
    #[clap(subcommand)]
    Show(show::Cmd),
    // /// Txs
    // #[clap(subcommand)]
    // Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        if let Cmd::Setup(cmd) = self {
            cmd.run()
        } else {
            let e = Env::load()?;
            let config = e.to_config()?;
            match self {
                Cmd::Show(cmd) => cmd.run(&config),
                // Cmd::Tx(cmd) => cmd.run(&config),
                Cmd::Setup(_) => panic!("Impossible"),
            }
        }
    }
}
