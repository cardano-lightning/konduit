use cardano_tx_builder::{Address, address::kind};

use dotenvy::dotenv;
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

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenv().ok();
        envy::prefixed("KONDUIT_").from_env::<Self>()
    }
}
