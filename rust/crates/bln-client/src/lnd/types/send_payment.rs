use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, base64::Base64, serde_as};
use std::collections::HashMap;

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
    pub payment_request: String,

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
    pub payment_route: super::graph_routes::Route,

    /// (bytes -> base64 string)
    /// The payment hash.
    #[serde_as(as = "Base64")]
    #[serde(default)]
    pub payment_hash: [u8; 32],
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RouterSendRequest {
    /// (bytes -> base64 string)
    /// The identity pubkey of the payment recipient.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<Vec<u8>>,

    /// (int64 -> string)
    /// The amount to send expressed in satoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt: Option<u64>,

    /// (bytes -> base64 string)
    /// The hash to use within the payment's HTLC.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_hash: Option<[u8; 32]>,

    /// (int32)
    /// The CLTV delta from the current height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_cltv_delta: Option<u64>,

    /// (string)
    /// A bare-bones invoice for a payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_request: Option<String>,

    /// (int32)
    /// The max number of seconds the payment should be pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,

    /// (int64 -> string)
    /// The maximum number of satoshis that will be paid as a fee.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_limit_sat: Option<u64>,

    /// (uint64 -> string)
    /// The channel id of the channel that must be taken to the first hop.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outgoing_chan_id: Option<u64>,

    /// (int32)
    /// An optional maximum total time lock for the route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cltv_limit: Option<u64>,

    // /// (RouteHint array)
    // /// An optional set of routing hints to assist in path finding.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub route_hints: Option<Vec<RouteHint>>,
    /// (map<uint64, bytes> -> map<string, base64 string>)
    /// An optional field that can be used to pass an arbitrary set of TLV records.
    #[serde_as(as = "Option<HashMap<DisplayFromStr, Base64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_custom_records: Option<HashMap<u64, Vec<u8>>>,

    /// (int64 -> string)
    /// The amount to send expressed in millisatoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt_msat: Option<u64>,

    /// (int64 -> string)
    /// The maximum number of millisatoshis that will be paid as a fee.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_limit_msat: Option<u64>,

    /// (bytes -> base64 string)
    /// The pubkey of the last hop of the route.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_hop_pubkey: Option<Vec<u8>>,

    /// (bool -> boolean)
    /// If set, circular payments to self are permitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_self_payment: Option<bool>,

    /// (FeatureBit[] -> string[])
    /// Features assumed to be supported by the final node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_features: Option<Vec<FeatureBit>>,

    /// (uint32)
    /// The maximum number of partial payments that may be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_parts: Option<u64>,

    /// (bool -> boolean)
    /// If set, inflight updates will not be streamed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_inflight_updates: Option<bool>,

    /// (uint64[] -> string[])
    /// The channel ids of the channels that must be taken to the first hop.
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outgoing_chan_ids: Option<Vec<u64>>,

    /// (bytes -> base64 string)
    /// The payment address of the generated invoice.
    #[serde_as(as = "Option<Base64>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_addr: Option<[u8; 32]>,

    /// (uint64 -> string)
    /// The maximum shard size in millisatoshis.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_shard_size_msat: Option<u64>,

    /// (bool -> boolean)
    /// If set, AMP-style payments are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amp: Option<bool>,

    /// (double)
    /// A preference for circuits with a higher probability of success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_pref: Option<f64>,

    /// (bool -> boolean)
    /// Indicates whether the payment attempt can be canceled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelable: Option<bool>,

    /// (map<uint64, bytes> -> map<string, base64 string>)
    /// An optional field that can be used to pass an arbitrary set of TLV records
    /// to the first hop.
    #[serde_as(as = "Option<HashMap<DisplayFromStr, Base64>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_hop_custom_records: Option<HashMap<u64, Vec<u8>>>,
}
