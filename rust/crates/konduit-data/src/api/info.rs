use crate::ChannelParameters;
use cardano_sdk::{Address, Hash, address::kind::Shelley};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Info {
    /// Terms of service. Purely informational
    #[n(0)]
    pub tos: TosInfo,
    /// Channel parameters
    #[n(1)]
    pub channel_parameters: ChannelParameters,
    // Tx building
    #[n(2)]
    pub tx_help: TxHelp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TosInfo {
    #[n(0)]
    pub flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TxHelp {
    #[cbor(n(0), with = "crate::cbor_with::display_from_str")]
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<Shelley>,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    pub validator: Hash<28>,
}
