use crate::config::connector::Connector;
use cardano_tx_builder::{Address, NetworkId, SigningKey, address::kind};
use core::fmt;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Config {
    pub connector: Connector,

    pub root: [u8; 32],

    pub host_address: Address<kind::Shelley>,

    pub accounts: usize,
}

impl Config {
    const LABEL: &str = "Hammer";
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let root_key = self.wallet().to_verification_key();
        let address = root_key.to_address(network_id);
        writeln!(f, "== {} ==", Self::LABEL)?;
        writeln!(f, "{}", self.connector)?;
        writeln!(f, "host_address = {}", self.host_address)?;
        writeln!(f, "own_address = {}", address)?;
        writeln!(f, "root_key = {}", root_key)?;
        writeln!(f, "accounts = {}", self.accounts)?;
        Ok(())
    }
}

impl Config {
    pub fn wallet(&self) -> SigningKey {
        SigningKey::from(self.root)
    }

    pub fn account_bytes(&self, index: usize) -> [u8; 32] {
        if index >= self.accounts {
            panic!("index greater that number of accounts ({})", index)
        }
        let mut derived = self.root;
        let index_bytes = index.to_le_bytes();
        for i in 0..index_bytes.len() {
            derived[i] ^= index_bytes[i];
        }
        derived
    }

    pub fn account(&self, index: usize) -> SigningKey {
        SigningKey::from(self.account_bytes(index))
    }
}
