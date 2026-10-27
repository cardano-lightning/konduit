//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL-2.0/.

use crate::{NetworkId, ProtocolParameters, cbor, cbor as minicbor};
use anyhow::anyhow;
use std::fmt;

#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(into = "String", try_from = "String"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, cbor::Encode, cbor::Decode)]
pub enum PublicNetwork {
    #[n(0)]
    Mainnet,
    #[n(1)]
    Preprod,
    #[n(2)]
    Preview,
}

impl fmt::Display for PublicNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Self::Mainnet => "mainnet",
            Self::Preprod => "preprod",
            Self::Preview => "preview",
        })
    }
}

impl From<PublicNetwork> for u64 {
    fn from(network: PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => Network::MAINNET.magic(),
            PublicNetwork::Preprod => Network::PREPROD.magic(),
            PublicNetwork::Preview => Network::PREVIEW.magic(),
        }
    }
}

impl From<PublicNetwork> for ProtocolParameters {
    fn from(network: PublicNetwork) -> ProtocolParameters {
        match network {
            PublicNetwork::Mainnet => Self::mainnet(),
            PublicNetwork::Preprod => Self::preprod(),
            PublicNetwork::Preview => Self::preview(),
        }
    }
}

impl From<PublicNetwork> for NetworkId {
    fn from(network: PublicNetwork) -> NetworkId {
        match network {
            PublicNetwork::Mainnet => NetworkId::MAINNET,
            _ => NetworkId::TESTNET,
        }
    }
}

impl From<PublicNetwork> for String {
    fn from(network: PublicNetwork) -> Self {
        network.to_string()
    }
}

impl TryFrom<&str> for PublicNetwork {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        fn match_str(candidate: &str, target: PublicNetwork) -> bool {
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

impl std::str::FromStr for PublicNetwork {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<String> for PublicNetwork {
    type Error = anyhow::Error;

    fn try_from(text: String) -> anyhow::Result<Self> {
        Self::try_from(text.as_str())
    }
}

impl TryFrom<u64> for PublicNetwork {
    type Error = UnknownPublicNetwork;

    fn try_from(magic: u64) -> Result<Self, Self::Error> {
        Network::new(magic).try_into()
    }
}

impl PublicNetwork {
    pub fn is_mainnet(&self) -> bool {
        self == &PublicNetwork::Mainnet
    }

    pub fn is_testnet(&self) -> bool {
        !self.is_mainnet()
    }
}

/// A Cardano network identified by its magic number which facilitates private testnet usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Network(u64);

impl Network {
    pub const MAINNET_MAGIC: u64 = 764_824_073;
    pub const PREPROD_MAGIC: u64 = 1;
    pub const PREVIEW_MAGIC: u64 = 2;

    pub const MAINNET: Self = Self(Self::MAINNET_MAGIC);
    pub const PREPROD: Self = Self(Self::PREPROD_MAGIC);
    pub const PREVIEW: Self = Self(Self::PREVIEW_MAGIC);

    pub const fn new(magic: u64) -> Self {
        Self(magic)
    }

    pub const fn magic(self) -> u64 {
        self.0
    }
}

impl From<PublicNetwork> for Network {
    fn from(network: PublicNetwork) -> Self {
        Self::new(u64::from(network))
    }
}

impl TryFrom<Network> for PublicNetwork {
    type Error = UnknownPublicNetwork;

    fn try_from(network: Network) -> Result<Self, Self::Error> {
        match network.magic() {
            Network::MAINNET_MAGIC => Ok(PublicNetwork::Mainnet),
            Network::PREPROD_MAGIC => Ok(PublicNetwork::Preprod),
            Network::PREVIEW_MAGIC => Ok(PublicNetwork::Preview),
            magic => Err(UnknownPublicNetwork(magic)),
        }
    }
}

impl From<Network> for u64 {
    fn from(network: Network) -> u64 {
        network.magic()
    }
}

impl From<u64> for Network {
    fn from(magic: u64) -> Self {
        Self::new(magic)
    }
}

impl From<Network> for NetworkId {
    fn from(network: Network) -> NetworkId {
        if network.try_into() == Ok(PublicNetwork::Mainnet) {
            NetworkId::MAINNET
        } else {
            NetworkId::TESTNET
        }
    }
}

impl From<Network> for String {
    fn from(network: Network) -> Self {
        network.to_string()
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TryInto::<PublicNetwork>::try_into(*self) {
            Ok(public) => public.fmt(f),
            _ => write!(f, "{}", self.0),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Network {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Network {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>().map_err(serde::de::Error::custom)
    }
}

impl std::str::FromStr for Network {
    type Err = ParseNetworkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(public) = s.parse::<PublicNetwork>() {
            return Ok(Network::from(public));
        }
        s.parse::<u64>()
            .map(Network::new)
            .map_err(|_| ParseNetworkError)
    }
}

impl<C> cbor::Encode<C> for Network {
    fn encode<W: cbor::encode::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.u64(self.0)?;
        Ok(())
    }
}

impl<'b, C> cbor::Decode<'b, C> for Network {
    fn decode(d: &mut cbor::Decoder<'b>, _: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self::new(d.u64()?))
    }
}

// ============================================================== Error types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownPublicNetwork(pub u64);

impl fmt::Display for UnknownPublicNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown public network with magic {}", self.0)
    }
}

impl std::error::Error for UnknownPublicNetwork {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseNetworkError;

impl fmt::Display for ParseNetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "expected a public network name (mainnet/preprod/preview) or a decimal magic u64",
        )
    }
}

impl std::error::Error for ParseNetworkError {}
