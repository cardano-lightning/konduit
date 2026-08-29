//! Syncing tip data from the chain. Nothing does this implicitly -
//! `tip at` only ever reads what's already cached.

use cardano_connector::CardanoConnector;
use cardano_wallet::Wallet as WalletTrait;

use crate::Session;

use super::cmd::print_json;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Refresh every currently-tracked address.
    All,
    /// Refreshes each input (label or address), tracking any not
    /// already tracked. All inputs are resolved before any refresh
    /// happens, so an unrecognized one fails the whole call.
    At {
        #[arg(required = true)]
        label_or_address: Vec<String>,
    },
}

impl Cmd {
    pub async fn run<C: CardanoConnector, W: WalletTrait>(
        &self,
        session: &mut Session<C, W>,
    ) -> anyhow::Result<()> {
        match self {
            Cmd::All => {
                session.refresh_all().await?;
                print_json(&serde_json::json!({ "refreshed": "all" }))
            }
            Cmd::At { label_or_address } => {
                let addresses = label_or_address
                    .iter()
                    .map(|input| session.resolve(input))
                    .collect::<Result<Vec<_>, _>>()?;
                session.refresh_many(addresses).await?;
                print_json(&serde_json::json!({ "refreshed": label_or_address }))
            }
        }
    }
}
