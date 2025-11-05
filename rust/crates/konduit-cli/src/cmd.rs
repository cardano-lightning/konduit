mod data;
mod tx;
mod wallet;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
#[clap(propagate_version = true)]
pub(crate) enum Cmd {
    #[clap(subcommand)]
    Data(data::Cmd),

    #[clap(subcommand)]
    Wallet(wallet::Cmd),

    #[clap(subcommand)]
    Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Data(cmd) => cmd.execute().await,
            Self::Wallet(cmd) => cmd.execute().await,
            Self::Tx(cmd) => cmd.execute().await,
        }
    }
}
