use crate::config::connector::Connector;
use cardano_tx_builder::{Address, NetworkId, SigningKey, address::kind};
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
        write!(f, "== {} ==\n", Self::LABEL)?;
        write!(f, "{}\n", self.connector)?;
        write!(f, "address = {}\n", address)?;
        write!(f, "host_address = {}\n", self.host_address)?;
        write!(f, "adaptor_key = {}\n", vk)?;
        write!(f, "close_period = {}\n", self.close_period)?;
        write!(f, "fee = {}\n", self.fee)?;
        Ok(())
    }
}
