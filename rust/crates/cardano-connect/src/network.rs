use cardano_tx_builder::NetworkId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum Network {
    Mainnet,
    Preview,
    Preprod,
    Other(u64),
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
