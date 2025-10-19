// mod tx;
mod cardano;
mod metavar;
mod data;
mod wallet;

#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Txs
    // #[command(subcommand)]
    // Tx(tx::Cmd),
    #[command(subcommand)]
    Data(data::Cmd),
    #[command(subcommand)]
    Cardano(cardano::Cmd),
    #[command(subcommand)]
    Wallet(wallet::Cmd),
}

impl Cmd {
    pub fn run(self: Self) {
        match self {
            // Cmd::Tx(cmd) => cmd.run(),
            Cmd::Data(cmp) => cmd.run(),
            Cmd::Cardano(cmd) => cmd.run(),
            Cmd::Wallet(cmd) => cmd.run(),
        };
    }
}
