//! # Config
//!
//! Pieces together the config of different components
use std::collections::BTreeMap;

use konduit_data::Tag;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{l1, l2, server};

#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// Set if there is an embedded wallet
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<Option<serde_with::hex::Hex>>")
    )]
    wallet: Option<[u8; 32]>,
    /// Set if there is an embedded signer `add_vkey`.
    #[n(1)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<Option<serde_with::hex::Hex>>")
    )]
    signer: Option<[u8; 32]>,
    /// L1 config
    #[n(2)]
    l1: l1::Config,
    /// Known konduit servers
    #[n(3)]
    servers: Vec<server::Config>,
    /// L2 configs which may or may not use a base_url from the known servers.
    #[n(4)]
    l2s: BTreeMap<Tag, l2::Config>,
}
