use crate::ChannelParameters;
use cardano_sdk::{Address, Hash, address::kind::Shelley};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptorInfo<T> {
    // Terms of service. Purely informational
    pub tos: TosInfo,
    // Channel parameters
    pub channel_parameters: ChannelParameters,
    // Tx building
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TosInfo {
    pub flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxHelp {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub host_address: Address<Shelley>,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub validator: Hash<28>,
}
