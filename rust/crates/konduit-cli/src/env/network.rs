use std::fmt;

use serde::{Deserialize, Serialize};

/// This hint allows setup to generate the right addresses,
/// without specifying the connector (from which it can be inferred)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize, clap::ValueEnum,
)]
pub enum Network {
    Mainnet,
    Preview,
    Preprod,
    Custom,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(match self {
            Network::Mainnet => "mainnet",
            Network::Preview => "preview",
            Network::Preprod => "preprod",
            Network::Custom => "custom",
        })
    }
}
