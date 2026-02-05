use core::fmt;
use std::fmt::Display;

use cardano_tx_builder::{Address, NetworkId, address::kind};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::config::{connector::Connector, signing_key::SigningKey};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connector: Connector,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub wallet: SigningKey,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<kind::Shelley>,
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let address = self.wallet.to_address(&network_id);
        let key = self.wallet.to_verification_key();
        write!(f, "== {} ==\n", Self::LABEL)?;
        write!(f, "{}\n", self.connector)?;
        write!(f, "host_address = {}\n", self.host_address)?;
        write!(f, "own_address = {}\n", address)?;
        write!(f, "own_key = {}\n", key)?;
        Ok(())
    }
}

impl Config {
    const LABEL: &str = "Consumer";
}
