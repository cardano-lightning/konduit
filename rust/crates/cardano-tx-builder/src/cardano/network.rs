//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{NetworkId, pallas};
use std::fmt;

/// Network distinguishes mainnet from testsnets, like `Network`,
/// but also the various testnets, unlike `Network`.
/// This is helpful when handling slot configs amongst other things.
///
/// `Network` includes mainnet, two commonly used testnets,
/// with the option of custom devnets.
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
            Network::Mainnet => NetworkId::mainnet(),
            _ => NetworkId::testnet(),
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

impl From<Network> for pallas::Network {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => pallas::Network::Mainnet,
            Network::Preview => pallas::Network::Testnet,
            Network::Preprod => pallas::Network::Testnet,
            Network::Other(n) => pallas::Network::Other(n as u8),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::Network;
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_mainnet() {
        assert_eq!(Network::mainnet().to_string(), "mainnet")
    }

    #[test]
    fn display_preview() {
        assert_eq!(Network::preview().to_string(), "preview")
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn network()(is_mainnet in any::<bool>()) -> Network {
                if is_mainnet {
                    Network::mainnet()
                } else {
                    Network::other(222)
                }
            }
        }
    }
}
