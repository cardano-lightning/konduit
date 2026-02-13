use crate::{
    config::{adaptor::Config, connector::Connector, signing_key::SigningKey},
    env::{base::signing_key_to_address, connector},
    shared::{DefaultPath, Fill},
};
use cardano_tx_builder::{Address, NetworkId, address::kind};
use connector::ConnectorEnv;
use konduit_data::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct Env {
    /// Blockfrost project id
    #[command(flatten)]
    #[serde(flatten)]
    pub connector: ConnectorEnv,

    // Wallet signing key (32 byte hex)
    #[arg(long, env = "KONDUIT_WALLET")]
    #[serde(rename = "KONDUIT_WALLET")]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pub wallet: Option<SigningKey>,

    /// Address of Konduit reference script
    /// This `fill`s to admin wallets address (with no delegation).
    #[arg(long, env = "KONDUIT_HOST_ADDRESS")]
    #[serde(rename = "KONDUIT_HOST_ADDRESS")]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub host_address: Option<Address<kind::Shelley>>,

    /// (Minimum acceptable) Close period
    #[arg(long, env = "KONDUIT_CLOSE_PERIOD")]
    #[serde(rename = "KONDUIT_CLOSE_PERIOD")]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub close_period: Option<Duration>,

    /// (Flat) fee (lovelace)
    #[arg(long, env = "KONDUIT_FEE")]
    #[serde(rename = "KONDUIT_FEE")]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub fee: Option<u64>,
}

impl TryFrom<Env> for Config {
    type Error = anyhow::Error;

    fn try_from(env: Env) -> Result<Self, Self::Error> {
        let connector = Connector::try_from(env.connector)?;

        let wallet = env.wallet.ok_or(anyhow::anyhow!("Wallet required"))?;

        let host_address = env
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?;

        let close_period = env
            .close_period
            .ok_or(anyhow::anyhow!("Close period required"))?;

        let fee = env.fee.ok_or(anyhow::anyhow!("Fee required"))?;

        Ok(Config {
            connector,
            wallet,
            host_address,
            close_period,
            fee,
        })
    }
}

impl DefaultPath for Env {
    const DEFAULT_PATH: &str = ".env.adaptor";
}

impl Fill for Env {
    fn fill(self) -> Self {
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
}
