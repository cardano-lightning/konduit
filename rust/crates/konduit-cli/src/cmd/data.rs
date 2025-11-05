mod cheque;
mod squash;

/// Konduit data manipulation
#[derive(clap::Subcommand)]
pub enum Cmd {
    Cheque(cheque::Args),
    Squash(squash::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Cmd::Cheque(cmd) => cmd.execute(),
            Cmd::Squash(cmd) => cmd.execute(),
        }
    }
}
