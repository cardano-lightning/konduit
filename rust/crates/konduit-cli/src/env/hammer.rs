use crate::{
    cmd::parsers::parse_hex_32,
    config::{connector::Connector, hammer::Config},
    env::{base::default_wallet_and_address, connector},
    shared::{DefaultPath, Fill, Setup},
};
use cardano_tx_builder::{Address, address::kind};
use connector::ConnectorEnv;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_with::{IfIsHumanReadable, hex::Hex, serde_as};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct Env {
    /// Blockfrost project id
    #[command(flatten)]
    #[serde(flatten)]
    pub connector: ConnectorEnv,

    /// "Root" Wallet signing key (32 byte hex)
    #[arg(long, env = "KONDUIT_ROOT", value_parser= parse_hex_32 )]
    #[serde(rename = "KONDUIT_ROOT")]
    #[serde_as(as = "Option<IfIsHumanReadable<Hex, _>>")]
    pub root: Option<[u8; 32]>,

    /// Address of Konduit reference script
    #[arg(long, env = "KONDUIT_HOST_ADDRESS")]
    #[serde(rename = "KONDUIT_HOST_ADDRESS")]
    #[serde_as(as = "Option<serde_with::DisplayFromStr>")]
    pub host_address: Option<Address<kind::Shelley>>,

    /// Number of accounts
    #[arg(long, env = "KONDUIT_ACCOUNTS", default_value_t = 10)]
    #[serde(rename = "KONDUIT_ACCOUNTS")]
    pub accounts: usize,
}

impl TryFrom<Env> for Config {
    type Error = anyhow::Error;

    fn try_from(env: Env) -> Result<Self, Self::Error> {
        let connector = Connector::try_from(env.connector)?;

        let root = env.root.ok_or(anyhow::anyhow!("Wallet required"))?;

        let host_address = env
            .host_address
            .ok_or(anyhow::anyhow!("Host address required"))?;

        Ok(Config {
            connector,
            root,
            host_address,
            accounts: env.accounts,
        })
    }
}

impl Setup for Env {}

impl DefaultPath for Env {
    const DEFAULT_PATH: &str = ".env.hammer";
}

impl Fill for Env {
    type Error = anyhow::Error;

    fn fill(self) -> anyhow::Result<Self> {
        let connector = self.connector.fill()?;

        let root = random_root();
        let (_wallet, host_address) =
            default_wallet_and_address(connector.network_id(), None, None);

        Ok(Self {
            connector,
            root: Some(root),
            host_address: Some(host_address),
            accounts: 10,
        })
    }
}

fn random_root() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    bytes
}
