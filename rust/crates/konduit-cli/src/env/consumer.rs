use cardano_tx_builder::{Address, NetworkId, address::kind};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    config::{consumer::Config, signing_key::SigningKey},
    env::base::{load, load_dotenv, signing_key_to_address},
};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct Env {
    /// Blockfrost project id
    #[command(flatten)]
    #[serde(flatten)]
    pub connector: super::connector::ConnectorEnv,
    // Wallet signing key (32 byte hex)
    #[arg(long)]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    #[serde(rename = "KONDUIT_WALLET")]
    pub wallet: Option<SigningKey>,
    /// Address of Konduit reference script
    #[arg(long)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(rename = "KONDUIT_HOST_ADDRESS")]
    pub host_address: Option<Address<kind::Shelley>>,
}

impl Env {
    pub const DEFAULT_PATH: &str = ".env.consumer";

    /// Insert generated or placeholder values
    pub fn fill(self) -> Self {
        let connector = self.connector.fill();
        let network_id = connector.network_id().unwrap_or(NetworkId::MAINNET);
        let wallet = self.wallet.unwrap_or(SigningKey::generate());
        let host_address = self
            .host_address
            .unwrap_or(signing_key_to_address(&network_id, &wallet));
        Self {
            connector,
            wallet: Some(wallet),
            host_address: Some(host_address),
        }
    }

    pub fn to_config(self) -> anyhow::Result<Config> {
        let connector = self.connector.to_config()?;
        let wallet = self.wallet.ok_or(anyhow::anyhow!("Wallet required"))?;
        let host_address = self
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?;
        let config = Config {
            connector,
            wallet,
            host_address,
        };
        Ok(config)
    }

    pub fn load() -> anyhow::Result<Self> {
        load_dotenv(Self::DEFAULT_PATH)?;
        load::<Self>()
    }
}
