use anyhow::anyhow;
use cardano_tx_builder::NetworkId;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Network {
    Mainnet,
    Preview,
    Preprod,
    Other(u64),
}

type NetworkMagicNumber = u64;

const MAINNET_MAGIC_NUMBER: u64 = 764824073;
const PREPROD_MAGIC_NUMBER: u64 = 1;
const PREVIEW_MAGIC_NUMBER: u64 = 2;

impl Into<NetworkMagicNumber> for Network {
    fn into(self) -> u64 {
        match self {
            Network::Mainnet => MAINNET_MAGIC_NUMBER,
            Network::Preprod => PREPROD_MAGIC_NUMBER,
            Network::Preview => PREVIEW_MAGIC_NUMBER,
            Network::Other(n) => n,
        }
    }
}

impl From<Network> for NetworkId {
    fn from(network: Network) -> NetworkId {
        match network {
            Network::Mainnet => NetworkId::MAINNET,
            _ => NetworkId::TESTNET,
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Network::Mainnet => "mainnet",
            Network::Preview => "preview",
            Network::Preprod => "preprod",
            Network::Other(_n) => "other",
        })
    }
}

impl TryFrom<&str> for Network {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        match text {
            mainnet if mainnet == Network::Mainnet.to_string() => Ok(Network::mainnet()),
            preview if preview == Network::Preview.to_string() => Ok(Network::preview()),
            preprod if preprod == Network::Preprod.to_string() => Ok(Network::preprod()),
            _ => Err(anyhow!("Unknown network not yet supported")),
        }
    }
}

// -------------------------------------------------------------------- Building

impl Network {
    pub fn mainnet() -> Self {
        Network::Mainnet
    }

    pub fn preview() -> Self {
        Network::Preview
    }

    pub fn preprod() -> Self {
        Network::Preprod
    }

    pub fn other(n: u64) -> Self {
        Network::Other(n)
    }
}

// ------------------------------------------------------------------ Inspecting

impl Network {
    pub fn is_mainnet(&self) -> bool {
        *self == Network::Mainnet
    }

    pub fn is_testnet(&self) -> bool {
        *self != Network::Mainnet
    }
}
