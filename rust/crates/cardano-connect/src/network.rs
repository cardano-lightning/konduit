use anyhow::anyhow;
use cardano_tx_builder::{NetworkId, ProtocolParameters};
use std::fmt;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum Network {
    Mainnet,
    Preview,
    Preprod,
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

impl TryFrom<&str> for Network {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        match text {
            mainnet if mainnet == Self::Mainnet.to_string() => Ok(Network::mainnet()),
            preprod if preprod == Self::Preprod.to_string() => Ok(Network::preprod()),
            preview if preview == Self::Preview.to_string() => Ok(Network::preview()),
            _ => Err(anyhow!(
                "unsupported network: {text}; should be one of {}, {}, {}",
                Self::Mainnet,
                Self::Preprod,
                Self::Preview
            )),
        }
    }
}

// -------------------------------------------------------------------- Building

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Network {
    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn mainnet() -> Self {
        Network::Mainnet
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn preprod() -> Self {
        Network::Preprod
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn preview() -> Self {
        Network::Preview
    }
}

// ------------------------------------------------------------------ Inspecting

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl Network {
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "isMainnet"))]
    pub fn is_mainnet(self) -> bool {
        self == Network::Mainnet
    }

    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "isTestnet"))]
    pub fn is_testnet(self) -> bool {
        self != Network::Mainnet
    }
}

// --------------------------------------------------------------- WASM-specific

#[cfg_attr(feature = "wasm", wasm_bindgen, doc(hidden))]
impl Network {
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "asMagic"))]
    pub fn _wasm_as_magic(self) -> u64 {
        u64::from(self)
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toString"))]
    pub fn _wasm_to_string(self) -> String {
        self.to_string()
    }
}
