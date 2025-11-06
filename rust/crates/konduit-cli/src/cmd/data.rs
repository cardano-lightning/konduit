mod mixed_cheque;
mod squash;

/// Konduit data manipulation
#[derive(clap::Subcommand)]
pub enum Cmd {
    MixedCheque(mixed_cheque::Args),
    Squash(squash::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Cmd::MixedCheque(cmd) => cmd.execute(),
            Cmd::Squash(cmd) => cmd.execute(),
        }
    }
}
