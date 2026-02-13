use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, base64::Base64, serde_as};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GraphRoutes {
    pub routes: Vec<Route>,
}

/// Represents a route in the Lightning Network.
/// Converted from serde-aux to serde_with.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Route {
    // This field was 'String' in the gRPC-gateway, but often it's an object
    // or base64 string in reality. Sticking with String as requested.
    // If it's bytes, it should be: #[serde_as(as = "Base64")] pub custom_channel_data: Vec<u8>,
    pub custom_channel_data: String,

    #[serde_as(as = "DisplayFromStr")]
    pub first_hop_amount_msat: u64,

    pub hops: Vec<Hop>,

    #[serde_as(as = "DisplayFromStr")]
    pub total_amt: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_amt_msat: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_fees: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub total_fees_msat: u64,

    pub total_time_lock: u64,
}

/// Represents a single hop in a route.
/// Converted from serde-aux to serde_with.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hop {
    //pub amp_record: Option<Value>, // Value not defined, keeping commented
    //pub mpp_record: Option<Value>, // Value not defined, keeping commented
    #[serde_as(as = "DisplayFromStr")]
    pub amt_to_forward: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub amt_to_forward_msat: u64,

    // Assuming these are base64 strings representing bytes, like other fields
    // If they are just strings, remove the serde_as.
    #[serde_as(as = "Base64")]
    pub blinding_point: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub encrypted_data: Vec<u8>,

    #[serde_as(as = "Base64")]
    pub metadata: Vec<u8>,

    #[serde_as(as = "DisplayFromStr")]
    pub chan_capacity: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub chan_id: u64,

    pub expiry: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee: u64,

    #[serde_as(as = "DisplayFromStr")]
    pub fee_msat: u64,

    /// The pubkey of the hop, 33 bytes.
    #[serde_as(as = "serde_with::hex::Hex")]
    pub pub_key: [u8; 33],

    pub tlv_payload: bool,

    #[serde_as(as = "DisplayFromStr")]
    #[serde(default)] // This field is sometimes missing, provide a default
    pub total_amt_msat: u64,
}
