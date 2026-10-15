use crate::config::connector::Connector;
use cardano_sdk::{Address, NetworkId, SigningKey, address::kind};
use core::fmt;
use std::{fmt::Display, path::PathBuf};
#[derive(Debug, Clone)]
pub struct Config {
    pub connector: Connector,
    pub wallet: SigningKey,
    pub host_address: Address<kind::Shelley>,
    pub asset_config: Option<PathBuf>,
}

impl Config {
    const LABEL: &str = "Consumer";
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let key = self.wallet.to_verification_key();
        let address = key.to_address(network_id);
        writeln!(f, "== {} ==", Self::LABEL)?;
        writeln!(f, "{}", self.connector)?;
        writeln!(f, "host_address = {}", self.host_address)?;
        if let Some(path) = &self.asset_config {
            writeln!(f, "asset_config = {}", path.display())?;
        }
        writeln!(f, "own_address = {}", address)?;
        writeln!(f, "own_key = {}", key)?;
        Ok(())
    }
}
