use crate::config::connector::Connector;
use cardano_sdk::{Address, NetworkId, SigningKey, address::kind};
use core::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Config {
    pub connector: Connector,

    pub wallet: SigningKey,

    pub host_address: Address<kind::Shelley>,
}

impl Config {
    const LABEL: &str = "Admin";
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let address = self.wallet.to_verification_key().to_address(network_id);
        writeln!(f, "== {} ==", Self::LABEL)?;
        writeln!(f, "{}", self.connector)?;
        writeln!(f, "address = {}", address)?;
        writeln!(f, "host_address = {}", self.host_address)?;
        Ok(())
    }
}
