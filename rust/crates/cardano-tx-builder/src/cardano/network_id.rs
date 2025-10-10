//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use anyhow::anyhow;
use std::fmt;

/// A network identifier to protect misuses of addresses or transactions on a wrong network.
///
/// Note that you can convert to and from [`u8`] using [`u8::from`] and [`Self::try_from`]
/// respectively.:
///
/// ```rust
/// # use cardano_tx_builder::{NetworkId};
/// assert_eq!(u8::from(NetworkId::testnet()), 0);
/// assert_eq!(u8::from(NetworkId::mainnet()), 1);
/// ```
///
/// ```rust
/// # use cardano_tx_builder::{NetworkId};
/// assert!(NetworkId::try_from(0_u8).is_ok_and(|network| network.is_testnet()));
/// assert!(NetworkId::try_from(1_u8).is_ok_and(|network| network.is_mainnet()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
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
    pub fn mainnet() -> Self {
        Self(pallas::NetworkId::Mainnet)
    }

    pub fn testnet() -> Self {
        Self(pallas::NetworkId::Testnet)
    }
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

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::NetworkId;
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_testnet() {
        assert_eq!(NetworkId::testnet().to_string(), "testnet")
    }

    #[test]
    fn display_mainnet() {
        assert_eq!(NetworkId::mainnet().to_string(), "mainnet")
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn network_id()(is_testnet in any::<bool>()) -> NetworkId {
                if is_testnet {
                    NetworkId::testnet()
                } else {
                    NetworkId::mainnet()
                }
            }
        }
    }
}
