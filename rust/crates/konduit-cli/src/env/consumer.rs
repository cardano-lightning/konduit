use crate::{
    config::{connector::Connector, consumer::Config, signing_key::SigningKey},
    env::{base::signing_key_to_address, connector},
    shared::{DefaultPath, Fill},
};
use cardano_tx_builder::{Address, NetworkId, address::kind};
use connector::ConnectorEnv;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct Env {
    /// Blockfrost project id
    #[command(flatten)]
    #[serde(flatten)]
    pub connector: ConnectorEnv,

    /// Wallet signing key (32 byte hex)
    #[arg(long, env = "KONDUIT_WALLET")]
    #[serde(rename = "KONDUIT_WALLET")]
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pub wallet: Option<SigningKey>,

    /// Address of Konduit reference script
    #[arg(long, env = "KONDUIT_HOST_ADDRESS")]
    #[serde(rename = "KONDUIT_HOST_ADDRESS")]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub host_address: Option<Address<kind::Shelley>>,
}

impl TryFrom<Env> for Config {
    type Error = anyhow::Error;

    fn try_from(env: Env) -> Result<Self, Self::Error> {
        let connector = Connector::try_from(env.connector)?;

        let wallet = env.wallet.ok_or(anyhow::anyhow!("Wallet required"))?;

        let host_address = env
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?;

        Ok(Config {
            connector,
            wallet,
            host_address,
        })
    }
}

impl DefaultPath for Env {
    const DEFAULT_PATH: &str = ".env.consumer";
}

impl Fill for Env {
    fn fill(self) -> Self {
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
}
