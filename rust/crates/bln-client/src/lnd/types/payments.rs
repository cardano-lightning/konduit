use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, base64::Base64, hex::Hex, serde_as};
use std::collections::HashMap;

/// Request parameters for GET /v1/payments
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Request {
    pub include_incomplete: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub index_offset: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub max_payments: Option<u64>,

    pub reversed: bool,
    pub count_total_payments: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub creation_date_start: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub creation_date_end: Option<u64>,
}

/// Response from GET /v1/payments
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub payments: Vec<Payment>,

    #[serde_as(as = "DisplayFromStr")]
    pub first_index_offset: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub last_index_offset: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_num_payments: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Payment {
    #[serde_as(as = "Hex")]
    pub payment_hash: [u8; 32],

    #[serde_as(as = "DisplayFromStr")]
    pub value_sat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub value_msat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub creation_time_ns: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee_sat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee_msat: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Hex>")]
    pub payment_preimage: Option<[u8; 32]>,

    pub payment_request: String,
    pub status: String, // "IN_FLIGHT", "SUCCEEDED", "FAILED"

    #[serde_as(as = "DisplayFromStr")]
    pub payment_index: u64,

    pub htlcs: Vec<HtlcAttempt>,
    pub failure_reason: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HtlcAttempt {
    #[serde_as(as = "DisplayFromStr")]
    pub attempt_id: u64,

    pub status: String, // "IN_FLIGHT", "SUCCEEDED", "FAILED"
    pub route: Route,

    #[serde_as(as = "DisplayFromStr")]
    pub attempt_time_ns: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub resolve_time_ns: u64,

    pub failure: Option<Failure>,

    /// NOTE: Inside HTLCs, LND encodes the preimage as Base64, unlike the root payment which is Hex!
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Base64>")]
    pub preimage: Option<Vec<u8>>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Route {
    /// NOTE: In REST, LND sends this as an actual JSON number, not a string.
    pub total_time_lock: u32,

    #[serde_as(as = "DisplayFromStr")]
    pub total_fees: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_amt: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_fees_msat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_amt_msat: u64,

    pub hops: Vec<Hop>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hop {
    #[serde_as(as = "DisplayFromStr")]
    pub chan_id: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub chan_capacity: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_to_forward: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee: u64,

    /// NOTE: In REST, LND sends this as an actual JSON number.
    pub expiry: u32,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_to_forward_msat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee_msat: u64,

    pub pub_key: String,
    pub tlv_payload: bool,

    // Using String keys for custom_records as JSON keys are always strings
    pub custom_records: HashMap<String, String>,

    pub mpp_record: Option<MppRecord>,
    pub amp_record: Option<AmpRecord>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MppRecord {
    #[serde_as(as = "Base64")]
    pub payment_addr: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    pub total_amt_msat: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AmpRecord {
    #[serde_as(as = "Base64")]
    pub root_share: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub set_id: Vec<u8>,

    pub child_index: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Failure {
    pub code: String,
    pub channel_update: Option<ChannelUpdate>,
    pub htlc_msat: String,
    pub onion_sha_256: String,
    pub cltv_expiry: u32,
    pub index: u32,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChannelUpdate {
    pub signature: String,
    pub chain_hash: String,

    #[serde_as(as = "DisplayFromStr")]
    pub chan_id: u64,

    pub timestamp: u32,
    pub message_flags: u32,
    pub channel_flags: u32,
    pub time_lock_delta: u32,

    #[serde_as(as = "DisplayFromStr")]
    pub htlc_minimum_msat: u64,

    pub base_fee: u32,
    pub fee_rate: u32,

    #[serde_as(as = "DisplayFromStr")]
    pub htlc_maximum_msat: u64,
}
