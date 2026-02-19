use anyhow::anyhow;
use cardano_tx_builder::{NetworkId, ProtocolParameters, cbor, cbor as minicbor};
use std::fmt;

#[cfg(feature = "wasm")]
use std::ops::Deref;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    cbor::Encode,
    cbor::Decode,
)]
#[serde(into = "String", try_from = "&str")]
pub enum Network {
    #[n(0)]
    Mainnet,
    #[n(1)]
    Preprod,
    #[n(2)]
    Preview,
}

pub const MAINNET_MAGIC: u64 = 764824073;
pub const PREPROD_MAGIC: u64 = 1;
pub const PREVIEW_MAGIC: u64 = 2;

impl From<Network> for u64 {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PREPROD_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
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

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Preprod => "preprod",
            Self::Preview => "preview",
        })
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

// --------------------------------------------------------------- WASM-specific

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
    cbor::Encode,
    cbor::Decode,
)]
pub struct NetworkName(#[n(0)] Network);

#[cfg(feature = "wasm")]
impl From<Network> for NetworkName {
    fn from(network: Network) -> Self {
        Self(network)
    }
}

#[cfg(feature = "wasm")]
impl Deref for NetworkName {
    type Target = Network;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl NetworkName {
    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn _wasm_new(network: &str) -> Result<Self, String> {
        Ok(Self(Network::try_from(network).map_err(|e| e.to_string())?))
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "isMainnet"))]
    pub fn _wasm_is_mainnet(&self) -> bool {
        self.is_mainnet()
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "isTestnet"))]
    pub fn _wasm_is_testnet(&self) -> bool {
        self.is_testnet()
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toMagic"))]
    pub fn _wasm_to_magic(&self) -> u64 {
        u64::from(self.0)
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toNetworkId"))]
    pub fn _wasm_to_network_id(&self) -> NetworkId {
        NetworkId::from(self.0)
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "displayName"))]
    pub fn _wasm_display_name(&self) -> String {
        self.to_string()
    }
}
