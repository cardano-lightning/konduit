use cardano_tx_builder::{Address, NetworkId, address::kind};
use konduit_data::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    config::{adaptor::Config, signing_key::SigningKey},
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
    /// This `fill`s to admin wallets address (with no delegation).
    #[arg(long)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(rename = "KONDUIT_HOST_ADDRESS")]
    pub host_address: Option<Address<kind::Shelley>>,
    /// (Minimum acceptable) Close period
    #[arg(long)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(rename = "KONDUIT_CLOSE_PERIOD")]
    pub close_period: Option<Duration>,
    /// (Flat) fee (lovelace)
    #[arg(long)]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    #[serde(rename = "KONDUIT_FEE")]
    pub fee: Option<u64>,
}

impl Env {
    pub const DEFAULT_PATH: &str = ".env.adaptor";

    /// Insert generated or placeholder values
    pub fn fill(self) -> Self {
        let connector = self.connector.fill();
        let network_id = connector.network_id().unwrap_or(NetworkId::MAINNET);
        let wallet = self.wallet.unwrap_or(SigningKey::generate());
        let host_address = self
            .host_address
            .unwrap_or(signing_key_to_address(&network_id, &wallet));
        let close_period = self
            .close_period
            .unwrap_or(Duration::from_secs(24 * 60 * 60));
        let fee = self.fee.unwrap_or(10_000);
        Self {
            connector,
            wallet: Some(wallet),
            host_address: Some(host_address),
            close_period: Some(close_period),
            fee: Some(fee),
        }
    }

    pub fn to_config(self) -> anyhow::Result<Config> {
        let connector = self.connector.to_config()?;
        let wallet = self.wallet.ok_or(anyhow::anyhow!("Wallet required"))?;
        let host_address = self
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?;
        let close_period = self
            .close_period
            .ok_or(anyhow::anyhow!("Close period required"))?;
        let fee = self.fee.ok_or(anyhow::anyhow!("Fee required"))?;
        let config = Config {
            connector,
            wallet,
            host_address,
            close_period,
            fee,
        };
        Ok(config)
    }

    pub fn load() -> anyhow::Result<Self> {
        load_dotenv(Self::DEFAULT_PATH)?;
        load::<Self>()
    }
}
