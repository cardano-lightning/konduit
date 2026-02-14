use crate::config::{connector::Connector, signing_key::SigningKey};
use cardano_tx_builder::{Address, NetworkId, address::kind};
use core::fmt;
use konduit_data::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::fmt::Display;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connector: Connector,

    #[serde_as(as = "serde_with::hex::Hex")]
    pub wallet: SigningKey,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<kind::Shelley>,

    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub close_period: Duration,

    pub fee: u64,
}

impl Config {
    const LABEL: &str = "Adaptor";
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let network_id = self.connector.network_id().unwrap_or(NetworkId::MAINNET);
        let address = self.wallet.to_address(&network_id);
        write!(f, "== {} ==\n", Self::LABEL)?;
        write!(f, "{}\n", self.connector)?;
        write!(f, "address = {}\n", address)?;
        write!(f, "host_address = {}\n", self.host_address)?;
        write!(f, "adaptor_key = {}\n", self.wallet.to_verification_key())?;
        write!(f, "close_period = {}\n", self.close_period)?;
        write!(f, "fee = {}\n", self.fee)?;
        Ok(())
    }
}
