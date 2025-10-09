//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use anyhow::anyhow;
use std::fmt;

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
