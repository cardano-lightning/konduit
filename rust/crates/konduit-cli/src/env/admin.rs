use crate::{
    config::{admin::Config, connector::Connector},
    env::{base::default_wallet_and_address, connector},
    shared::{DefaultPath, Fill, Setup},
};
use cardano_sdk::{Address, LeakableSigningKey, address::kind};
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
    pub wallet: Option<LeakableSigningKey>,

    /// Address of Konduit reference script
    /// This `fill`s to admin wallets address (with no delegation).
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
            wallet: wallet.into_signing_key(),
            host_address,
        })
    }
}

impl Setup for Env {}

impl DefaultPath for Env {
    const DEFAULT_PATH: &str = ".env.admin";
}

impl Fill for Env {
    type Error = anyhow::Error;

    fn fill(self) -> anyhow::Result<Self> {
        let connector = self.connector.fill()?;

        let (wallet, host_address) =
            default_wallet_and_address(connector.network_id()?, self.wallet, self.host_address);

        Ok(Self {
            connector,
            wallet: Some(wallet),
            host_address: Some(host_address),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Env;
    use crate::{config::connector::Backend, env::connector::ConnectorEnv, shared::Fill};
    use cardano_sdk::{Network, NetworkId, SigningKey};

    #[test]
    fn fill_derives_blockfrost_defaults_from_network_id() {
        let wallet = SigningKey::from([3; 32]);
        let env = Env {
            connector: ConnectorEnv {
                backend: Backend::Blockfrost,
                network: None,
                blockfrost_project_id: Some("preview12345".to_string()),
                utxorpc_uri: None,
            },
            wallet: Some(wallet.clone().into()),
            host_address: None,
        };

        let filled = env
            .fill()
            .expect("fill should derive Blockfrost host address");
        let expected = wallet.to_verification_key().to_address(NetworkId::TESTNET);

        assert_eq!(filled.connector.network, Some(Network::Preview));
        assert_eq!(filled.host_address.as_ref(), Some(&expected));
    }

    #[test]
    fn fill_keeps_utxorpc_explicit_network_without_blockfrost_fallback() {
        let wallet = SigningKey::from([4; 32]);
        let env = Env {
            connector: ConnectorEnv {
                backend: Backend::Utxorpc,
                network: Some(Network::Preprod),
                blockfrost_project_id: Some("preview12345".to_string()),
                utxorpc_uri: Some("http://127.0.0.1:1337".to_string()),
            },
            wallet: Some(wallet.clone().into()),
            host_address: None,
        };

        let filled = env.fill().expect("fill should preserve UTxO RPC network");
        let expected = wallet.to_verification_key().to_address(NetworkId::TESTNET);

        assert_eq!(filled.connector.network, Some(Network::Preprod));
        assert_eq!(filled.host_address.as_ref(), Some(&expected));
    }
}
