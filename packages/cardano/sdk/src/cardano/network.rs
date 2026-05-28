use crate::{NetworkId, ProtocolParameters, cbor, cbor as minicbor};
use anyhow::anyhow;
use std::fmt;

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "&str"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, cbor::Encode, cbor::Decode)]
pub enum Network {
    #[n(0)]
    Mainnet,
    #[n(1)]
    Preprod,
    #[n(2)]
    Preview,
}

impl Network {
    pub const MAINNET_MAGIC: u64 = 764824073;
    pub const PREPROD_MAGIC: u64 = 1;
    pub const PREVIEW_MAGIC: u64 = 2;
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Preprod => "preprod",
            Self::Preview => "preview",
        })
    }
}

impl From<Network> for u64 {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => Network::MAINNET_MAGIC,
            Network::Preprod => Network::PREPROD_MAGIC,
            Network::Preview => Network::PREVIEW_MAGIC,
        }
    }
}

impl From<Network> for ProtocolParameters {
    fn from(network: Network) -> ProtocolParameters {
        match network {
            Network::Mainnet => Self::mainnet(),
            Network::Preprod => Self::preprod(),
            Network::Preview => Self::preview(),
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

impl From<Network> for String {
    fn from(network: Network) -> Self {
        network.to_string()
    }
}

impl TryFrom<&str> for Network {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        fn match_str(candidate: &str, target: Network) -> bool {
            candidate.to_lowercase() == target.to_string()
        }

        match text {
            mainnet if match_str(mainnet, Self::Mainnet) => Ok(Self::Mainnet),
            preprod if match_str(preprod, Self::Preprod) => Ok(Self::Preprod),
            preview if match_str(preview, Self::Preview) => Ok(Self::Preview),
            _ => Err(anyhow!(
                "unsupported network: {text}; should be one of {}, {}, {}",
                Self::Mainnet,
                Self::Preprod,
                Self::Preview
            )),
        }
    }
}

// ------------------------------------------------------------------ Inspecting

impl Network {
    pub fn is_mainnet(&self) -> bool {
        self == &Network::Mainnet
    }

    pub fn is_testnet(&self) -> bool {
        !self.is_mainnet()
    }
}
