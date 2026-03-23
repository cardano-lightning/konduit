use bitcoin::secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Following the conventions of `lightning invoices` but including serde.

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RouteHint(pub Vec<RouteHintHop>);

impl From<lightning_invoice::RouteHint> for RouteHint {
    fn from(value: lightning_invoice::RouteHint) -> Self {
        Self(value.0.into_iter().map(|x| x.into()).collect::<Vec<_>>())
    }
}

impl From<RouteHint> for lightning_invoice::RouteHint {
    fn from(value: RouteHint) -> Self {
        Self(value.0.into_iter().map(|x| x.into()).collect::<Vec<_>>())
    }
}

#[serde_as]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct RouteHintHop {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub src_node_id: PublicKey,
    pub short_channel_id: u64,
    pub fees: RoutingFees,
    pub cltv_expiry_delta: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub htlc_minimum_msat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub htlc_maximum_msat: Option<u64>,
}

impl From<lightning_invoice::RouteHintHop> for RouteHintHop {
    fn from(value: lightning_invoice::RouteHintHop) -> Self {
        let lightning_invoice::RouteHintHop {
            src_node_id,
            short_channel_id,
            fees,
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        } = value;
        Self {
            src_node_id,
            short_channel_id,
            fees: RoutingFees::from(fees),
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        }
    }
}

impl From<RouteHintHop> for lightning_invoice::RouteHintHop {
    fn from(value: RouteHintHop) -> Self {
        let RouteHintHop {
            src_node_id,
            short_channel_id,
            fees,
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        } = value;
        Self {
            src_node_id,
            short_channel_id,
            fees: lightning_invoice::RoutingFees::from(fees),
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct RoutingFees {
    pub base_msat: u32,
    pub proportional_millionths: u32,
}

impl From<lightning_invoice::RoutingFees> for RoutingFees {
    fn from(value: lightning_invoice::RoutingFees) -> Self {
        let lightning_invoice::RoutingFees {
            base_msat,
            proportional_millionths,
        } = value;
        Self {
            base_msat,
            proportional_millionths,
        }
    }
}

impl From<RoutingFees> for lightning_invoice::RoutingFees {
    fn from(value: RoutingFees) -> Self {
        let RoutingFees {
            base_msat,
            proportional_millionths,
        } = value;
        Self {
            base_msat,
            proportional_millionths,
        }
    }
}
