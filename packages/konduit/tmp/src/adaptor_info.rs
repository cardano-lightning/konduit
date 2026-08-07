use crate::ChannelParameters;
use cardano_sdk::{Address, Hash, address::kind::Shelley};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct AdaptorInfo<T> {
    // Terms of service. Purely informational
    #[n(0)]
    pub tos: TosInfo,
    // Channel parameters
    #[n(1)]
    pub channel_parameters: ChannelParameters,
    // Tx building
    #[n(2)]
    pub tx_help: T,
}

impl From<AdaptorInfo<TxHelp>> for AdaptorInfo<()> {
    fn from(info: AdaptorInfo<TxHelp>) -> Self {
        Self {
            tos: info.tos,
            channel_parameters: info.channel_parameters,
            tx_help: (),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TosInfo {
    #[n(0)]
    pub flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct TxHelp {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[n(0)]
    pub host_address: Address<Shelley>,
    #[serde_as(as = "serde_with::hex::Hex")]
    #[n(1)]
    pub validator: Hash<28>,
}
