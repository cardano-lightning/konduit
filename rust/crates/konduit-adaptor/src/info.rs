use actix_web::{HttpRequest, HttpResponse, Responder, body::BoxBody};
use cardano_tx_builder::{Address, Hash, address::kind::Shelley};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::common::{ChannelParameters, CommonArgs};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    // Terms of service. Purely informational
    pub tos: TosInfo,
    // Channel parameters
    pub channel_parameters: ChannelParameters,
    // Tx building
    pub tx_help: TxHelp,
}

impl Info {
    pub fn from_args(args: CommonArgs) -> Self {}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TosInfo {
    flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxHelp {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    host_address: Address<Shelley>,
    #[serde_as(as = "serde_with::hex::Hex")]
    validator: Hash<28>,
}

impl Responder for Info {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        // Standardizing on 200 OK for info queries
        HttpResponse::Ok().json(self)
    }
}
