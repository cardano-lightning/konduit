use std::str;

use tokio::runtime::Runtime;

use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Address, Value, address::kind};

use konduit_tx::{self, KONDUIT_VALIDATOR};

use crate::{cardano::ADA, config::adaptor::Config};

/// Create and submit Konduit transactions
#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {}

impl Cmd {
    pub fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector()?;
        let own_address = config.wallet.to_address(&connector.network().into());
        match self {}
    }
}
