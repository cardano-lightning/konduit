use crate::config::connector::Connector;
use cardano_sdk::{Address, NetworkId, SigningKey, address::kind};
use core::fmt;
use konduit_data::Duration;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Config {
    pub connector: Connector,

    pub wallet: SigningKey,

    pub host_address: Address<kind::Shelley>,

    pub close_period: Duration,

    pub fee: u64,
}

impl Config {
    const LABEL: &str = "Adaptor";
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let vk = self.wallet.to_verification_key();
        let address = vk.to_address(network_id);
        writeln!(f, "== {} ==", Self::LABEL)?;
        writeln!(f, "{}", self.connector)?;
        writeln!(f, "address = {}", address)?;
        writeln!(f, "host_address = {}", self.host_address)?;
        writeln!(f, "adaptor_key = {}", vk)?;
        writeln!(f, "close_period = {}", self.close_period)?;
        writeln!(f, "fee = {}", self.fee)?;
        Ok(())
    }
}
