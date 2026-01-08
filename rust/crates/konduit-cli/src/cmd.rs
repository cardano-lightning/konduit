mod admin;
mod consumer;
// mod tx;
// mod wallet;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
pub(crate) enum Cmd {
    #[clap(subcommand)]
    Admin(admin::Cmd),

    #[clap(subcommand)]
    Consumer(consumer::Cmd),
    // #[clap(subcommand)]
    // Wallet(wallet::Cmd),

    // #[clap(subcommand)]
    // Tx(tx::Cmd),
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Admin(cmd) => cmd.run(),
            Self::Consumer(cmd) => cmd.run(),
            // Self::Wallet(cmd) => cmd.run().await,
            // Self::Tx(cmd) => cmd.run().await,
        }
    }
}
