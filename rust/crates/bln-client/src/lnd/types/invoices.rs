use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, base64::Base64, serde_as};
use std::collections::HashMap;

/// The main response from the GET /v1/invoice/{r_hash} endpoint.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invoice {
    pub memo: String,

    #[serde_as(as = "Base64")]
    pub r_preimage: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub r_hash: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    pub value: i64,

    #[serde_as(as = "DisplayFromStr")]
    pub value_msat: i64,

    pub settled: bool,

    #[serde_as(as = "DisplayFromStr")]
    pub creation_date: i64,

    #[serde_as(as = "DisplayFromStr")]
    pub settle_date: i64,

    pub payment_request: String,

    #[serde_as(as = "Base64")]
    pub description_hash: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    pub expiry: i64,

    pub fallback_addr: String,

    #[serde_as(as = "DisplayFromStr")]
    pub cltv_expiry: u64,

    pub route_hints: Vec<RouteHint>,
    pub private: bool,

    #[serde_as(as = "DisplayFromStr")]
    pub add_index: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub settle_index: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_paid: i64, // (deprecated)

    #[serde_as(as = "DisplayFromStr")]
    pub amt_paid_sat: i64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_paid_msat: i64,

    pub state: String, // <InvoiceState> e.g. "SETTLED", "CANCELED", "OPEN"
    pub htlcs: Vec<InvoiceHtlc>,
    pub features: HashMap<u32, Feature>,
    pub is_keysend: bool,

    #[serde_as(as = "Base64")]
    pub payment_addr: Vec<u8>,

    pub is_amp: bool,
    pub amp_invoice_state: Option<HashMap<String, AmpInvoiceState>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteHint {
    pub hop_hints: Vec<HopHint>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HopHint {
    pub node_id: String,

    #[serde_as(as = "DisplayFromStr")]
    pub chan_id: u64,

    pub fee_base_msat: u32,
    pub fee_proportional_millionths: u32,
    pub cltv_expiry_delta: u32,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InvoiceHtlc {
    #[serde_as(as = "DisplayFromStr")]
    pub chan_id: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub htlc_index: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_msat: u64,

    pub accept_height: u32,

    #[serde_as(as = "DisplayFromStr")]
    pub accept_time: i64,

    #[serde_as(as = "DisplayFromStr")]
    pub resolve_time: i64,

    pub expiry_height: u32,
    pub state: String, // <HtlcState>
    pub custom_records: HashMap<String, String>,

    #[serde_as(as = "DisplayFromStr")]
    pub mpp_total_amt_msat: u64,

    pub amp: Option<Amp>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AmpInvoiceState {
    pub state: String, // <InvoiceState>

    #[serde_as(as = "DisplayFromStr")]
    pub settle_index: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub settle_time: i64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_paid_msat: i64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Amp {
    #[serde_as(as = "Base64")]
    pub root_share: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub set_id: Vec<u8>,

    pub child_index: u32,

    #[serde_as(as = "Base64")]
    pub hash: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub preimage: Vec<u8>,
}
