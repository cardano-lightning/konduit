//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use anyhow::anyhow;
use std::{fmt, str::FromStr};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// A network identifier to protect misuses of addresses or transactions on a wrong network.
///
/// Note that you can convert to and from [`u8`] using [`u8::from`] and [`Self::try_from`]
/// respectively.:
///
/// ```rust
/// # use cardano_tx_builder::{NetworkId};
/// assert_eq!(u8::from(NetworkId::TESTNET), 0);
/// assert_eq!(u8::from(NetworkId::MAINNET), 1);
/// ```
///
/// ```rust
/// # use cardano_tx_builder::{NetworkId};
/// assert!(NetworkId::try_from(0_u8).is_ok_and(|network| network.is_testnet()));
/// assert!(NetworkId::try_from(1_u8).is_ok_and(|network| network.is_mainnet()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
#[cfg_attr(
    feature = "wasm",
    wasm_bindgen,
    doc = "A network identifier to protect misuses of addresses or transactions on a wrong network."
)]
pub struct NetworkId(#[n(0)] pallas::NetworkId);

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self.0 {
            pallas::NetworkId::Testnet => "testnet",
            pallas::NetworkId::Mainnet => "mainnet",
        })
    }
}

// -------------------------------------------------------------------- Building

impl NetworkId {
    pub const MAINNET: Self = Self(pallas::NetworkId::Mainnet);
    pub const TESTNET: Self = Self(pallas::NetworkId::Testnet);
}

// ------------------------------------------------------------------ Inspecting

impl NetworkId {
    pub fn is_mainnet(&self) -> bool {
        self.0 == pallas::NetworkId::Mainnet
    }

    pub fn is_testnet(&self) -> bool {
        self.0 == pallas::NetworkId::Testnet
    }
}

// ----------------------------------------------------------- Converting (from)

impl FromStr for NetworkId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            _ if s == Self::MAINNET.to_string() => Ok(Self::MAINNET),
            _ if s == Self::TESTNET.to_string() => Ok(Self::TESTNET),
            _ => Err(anyhow!(
                "unrecognised network id: must be either 'mainnet' or 'testnet'"
            )),
        }
    }
}

impl From<pallas::NetworkId> for NetworkId {
    fn from(network_id: pallas::NetworkId) -> Self {
        Self(network_id)
    }
}

impl From<pallas::Network> for NetworkId {
    fn from(network: pallas::Network) -> Self {
        match network {
            pallas_addresses::Network::Mainnet => Self(pallas::NetworkId::Mainnet),
            pallas_addresses::Network::Testnet | pallas_addresses::Network::Other(..) => {
                Self(pallas::NetworkId::Testnet)
            }
        }
    }
}

impl TryFrom<u8> for NetworkId {
    type Error = anyhow::Error;

    fn try_from(i: u8) -> anyhow::Result<Self> {
        pallas::NetworkId::try_from(i)
            .map_err(|()| anyhow!("invalid network identifer; expected either 0 or 1"))
            .map(NetworkId)
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<NetworkId> for pallas::NetworkId {
    fn from(network_id: NetworkId) -> Self {
        network_id.0
    }
}

impl From<NetworkId> for pallas::Network {
    fn from(network_id: NetworkId) -> Self {
        match network_id.0 {
            pallas::NetworkId::Mainnet => pallas::Network::Mainnet,
            pallas::NetworkId::Testnet => pallas::Network::Testnet,
        }
    }
}

impl From<NetworkId> for u8 {
    fn from(network_id: NetworkId) -> u8 {
        u8::from(network_id.0)
    }
}

// -------------------------------------------------------------------- WASM

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen, doc(hidden))]
impl NetworkId {
    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "mainnet"))]
    pub fn _wasm_mainnet() -> Self {
        Self::MAINNET
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "testnet"))]
    pub fn _wasm_testnet() -> Self {
        Self::TESTNET
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toString"))]
    pub fn _wasm_to_string(&self) -> String {
        format!("{self:#?}")
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::NetworkId;
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_testnet() {
        assert_eq!(NetworkId::TESTNET.to_string(), "testnet")
    }

    #[test]
    fn display_mainnet() {
        assert_eq!(NetworkId::MAINNET.to_string(), "mainnet")
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn network_id()(is_testnet in any::<bool>()) -> NetworkId {
                if is_testnet {
                    NetworkId::TESTNET
                } else {
                    NetworkId::MAINNET
                }
            }
        }
    }
}
