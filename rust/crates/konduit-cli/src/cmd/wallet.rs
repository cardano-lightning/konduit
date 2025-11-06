mod channels;
mod generate;
mod show;
mod utxos;

/// Wallet Api
#[derive(clap::Subcommand)]
pub enum Cmd {
    Channels(channels::Args),
    /// Generate a new Ed25519 private key
    Gen(generate::Args),
    /// Display useful pieces of informations about a known wallet
    Show(show::Args),
    /// Fetch UTxO entries at the wallet's address; requires `Cardano` connection
    Utxos(utxos::Args),
}

impl Cmd {
    pub(crate) async fn execute(self) -> anyhow::Result<()> {
        match self {
            Cmd::Channels(cmd) => cmd.execute(crate::connector::new()?).await,
            Cmd::Gen(cmd) => cmd.execute(),
            Cmd::Show(cmd) => cmd.execute(),
            Cmd::Utxos(cmd) => cmd.execute(crate::connector::new()?).await,
        }
    }
}
