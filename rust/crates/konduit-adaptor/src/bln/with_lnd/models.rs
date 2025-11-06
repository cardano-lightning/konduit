use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, base64::Base64, serde_as};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct GetInfo {
    pub block_height: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Routes {
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

// This `#[serde_as]` macro is the main driver from the `serde_with` crate.
// It allows us to specify per-field (or per-type) serde logic.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SendPaymentRequest {
    /// (bytes -> base64 string)
    /// The identity pubkey of the payment recipient.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<[u8; 33]>,

    /// (string)
    /// The hex-encoded identity pubkey of the payment recipient (Deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_string: Option<String>,

    /// (int64 -> string)
    /// The amount to send expressed in satoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt: Option<u64>,

    /// (int64 -> string)
    /// The amount to send expressed in millisatoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt_msat: Option<u64>,

    /// (bytes -> base64 string)
    /// The hash to use within the payment's HTLC.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<[u8; 32]>,

    /// (string)
    /// The hex-encoded hash to use within the payment's HTLC (Deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash_string: Option<String>,

    /// (string)
    /// A bare-bones invoice for a payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_request: Option<String>,

    /// The CLTV delta from the current height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_cltv_delta: Option<u64>,

    /// (FeeLimit object)
    /// The maximum number of satoshis that will be paid as a fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_limit: Option<FeeLimit>,

    /// (uint64 -> string)
    /// The channel id of the channel that must be taken to the first hop.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outgoing_chan_id: Option<u64>,

    /// (bytes -> base64 string)
    /// The pubkey of the last hop of the route.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_hop_pubkey: Option<Vec<u8>>,

    /// An optional maximum total time lock for the route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cltv_limit: Option<u64>,

    /// (map<uint64, bytes> -> map<string, base64 string>)
    /// An optional field that can be used to pass an arbitrary set of TLV records.
    #[serde_as(as = "Option<HashMap<DisplayFromStr, Base64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_custom_records: Option<HashMap<u64, Vec<u8>>>,

    /// (bool -> boolean)
    /// If set, circular payments to self are permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_self_payment: Option<bool>,

    /// (FeatureBit[] -> string[])
    /// Features assumed to be supported by the final node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_features: Option<Vec<FeatureBit>>,

    /// (bytes -> base64 string)
    /// The payment address of the generated invoice.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_addr: Option<[u8; 32]>,
}

/// Helper struct for the `fee_limit` field.
/// This represents a gRPC `oneof` field, so all fields are optional.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FeeLimit {
    /// (int64 -> string)
    /// The fixed fee limit in satoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed: Option<u64>,

    /// (int64 -> string)
    /// The fixed fee limit in millisatoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_msat: Option<u64>,

    /// (int64 -> string)
    /// The fee limit as a percentage of the amount.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u64>,
}

/// Helper enum for the `dest_features` field.
/// LND REST API serializes gRPC enums as their string names.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureBit {
    DataLossProtectReq,
    DataLossProtectOpt,
    InitialLogonReq,
    InitialLogonOpt,
    UpfrontShutdownScriptReq,
    UpfrontShutdownScriptOpt,
    GossipQueriesReq,
    GossipQueriesOpt,
    TlvOnionReq,
    TlvOnionOpt,
    StaticRemoteKeyReq,
    StaticRemoteKeyOpt,
    PaymentAddrReq,
    PaymentAddrOpt,
    MppReq,
    MppOpt,
    KeysendReq,
    KeysendOpt,
    // Add other feature bits as needed...
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SendPaymentResponse {
    /// (string)
    /// If non-empty, this field indicates a payment error.
    #[serde(default)] // Use default (empty string) if null or missing
    pub payment_error: String,

    /// (bytes -> base64 string)
    /// The payment preimage.
    #[serde_as(as = "Base64")]
    #[serde(default)]
    pub payment_preimage: [u8; 32],

    /// (Route object)
    /// The route taken by the payment.
    #[serde(default)]
    pub payment_route: Route,

    /// (bytes -> base64 string)
    /// The payment hash.
    #[serde_as(as = "Base64")]
    #[serde(default)]
    pub payment_hash: [u8; 32],
}
