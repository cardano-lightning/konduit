mod open;
mod wallet;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
#[clap(propagate_version = true)]
pub(crate) enum Cmd {
    Open(open::Args),

    #[clap(subcommand)]
    Wallet(wallet::Cmd),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Open(cmd) => cmd.execute(crate::connector::new()?).await,
            Self::Wallet(cmd) => cmd.execute().await,
        }
    }
}
