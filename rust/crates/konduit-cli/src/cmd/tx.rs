mod deploy;
mod open;

/// Create and submit Konduit transactions
#[derive(clap::Subcommand)]
pub enum Cmd {
    Open(open::Args),
    Deploy(deploy::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Cmd::Open(cmd) => cmd.execute(crate::connector::new()?).await,
            Cmd::Deploy(cmd) => cmd.execute(crate::connector::new()?).await,
        }
    }
}
