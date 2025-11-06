mod deploy;
mod open;
mod sub;

/// Create and submit Konduit transactions
#[derive(clap::Subcommand)]
pub enum Cmd {
    Open(open::Args),
    Deploy(deploy::Args),
    Sub(sub::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Cmd::Open(cmd) => cmd.execute(crate::connector::new()?).await,
            Cmd::Deploy(cmd) => cmd.execute(crate::connector::new()?).await,
            Cmd::Sub(cmd) => cmd.execute(crate::connector::new()?).await,
        }
    }
}
