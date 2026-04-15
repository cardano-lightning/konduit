use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::core::{ChannelParameters, TosInfo, TxHelp};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    /// Terms of service. Purely informational
    #[n(0)]
    pub tos: TosInfo,
    /// Channel parameters
    #[n(1)]
    pub channel_parameters: ChannelParameters,
    /// Tx building
    #[n(2)]
    pub tx_help: TxHelp,
}
