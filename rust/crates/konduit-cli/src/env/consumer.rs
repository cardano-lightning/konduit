use cardano_tx_builder::{Address, address::kind};
use serde::{Deserialize, Serialize};

use crate::config::{consumer::Config, signing_key::SigningKey};

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct Env {
    /// Blockfrost project id
    #[command(flatten)]
    #[serde(flatten)]
    pub connector: super::connector::ConnectorEnv,
    /// Wallet signing key (32 byte hex)
    #[arg(long)]
    pub wallet: Option<String>,
    /// Address of Konduit reference script
    #[arg(long)]
    pub host_address: Option<String>,
}

impl Env {
    /// Insert generated or placeholder values
    pub fn fill(self) -> Self {
        Self {
            connector: self.connector.fill(),
            wallet: None,
            host_address: None,
        }
    }

    pub fn to_config(self) -> anyhow::Result<Config> {
        let connector = self.connector.to_config()?;
        let wallet = self
            .wallet
            .ok_or(anyhow::anyhow!("Wallet required"))?
            .parse::<SigningKey>()?;
        let host_address = self
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?
            .parse::<Address<kind::Shelley>>()?;
        let config = Config {
            connector,
            wallet,
            host_address,
        };
        Ok(config)
    }

    pub fn load() -> anyhow::Result<Self> {
        if dotenvy::from_filename(".env.consumer").is_err() {
            dotenvy::from_filename(".env").ok();
        }
        let x = envy::from_env::<Env>()?;
        Ok(x)
    }
}
