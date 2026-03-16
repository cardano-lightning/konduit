use bln_sdk::types::RouteHintHop;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RouteHints {
    pub route_hints: Vec<RouteHint>,
}

impl From<Vec<bln_sdk::types::RouteHint>> for RouteHints {
    fn from(value: Vec<bln_sdk::types::RouteHint>) -> Self {
        Self {
            route_hints: value.into_iter().map(|x| x.into()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RouteHint {
    pub hop_hints: Vec<HopHint>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HopHint {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub node_id: [u8; 33],
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub chan_id: u64,
    pub fee_base_msat: u64,
    pub fee_proportional_millionths: u64,
    pub cltv_expiry_delta: u64,
}

impl From<bln_sdk::types::RouteHint> for RouteHint {
    fn from(hint: bln_sdk::types::RouteHint) -> Self {
        Self {
            hop_hints: hint.0.into_iter().map(HopHint::from).collect(),
        }
    }
}

impl From<RouteHintHop> for HopHint {
    fn from(value: RouteHintHop) -> Self {
        Self {
            node_id: value.src_node_id.serialize(),
            chan_id: value.short_channel_id,
            fee_base_msat: value.fees.base_msat as u64,
            fee_proportional_millionths: value.fees.proportional_millionths as u64,
            cltv_expiry_delta: value.cltv_expiry_delta as u64,
        }
    }
}

// Reverse is not actually needed

// impl From<HopHint> for bln_sdk::types::RouteHintHop {
//     fn from(value: HopHint) -> Self {
//         Self {
//             src_node_id: value.node_id,
//             short_channel_id: value.chan_id,
//             fees: bln_sdk::types::RoutingFees {
//                 base_msat: value.fee_base_msat as u32,
//                 proportional_millionths: value.fee_proportional_millionths as u32,
//             },
//             cltv_expiry_delta: value.cltv_expiry_delta as u16,
//             // FIXME :: Are these supported??
//             htlc_minimum_msat: None,
//             htlc_maximum_msat: None,
//         }
//     }
// }
