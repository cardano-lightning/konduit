//! BLN template: quote without a BOLT-11 invoice.
//!
//! The naming convention follows BOLT-11 spec.
//!
//! The client specifies the payment parameters directly.
//! The lock (`r_hash`) will be taken from the cheque.
//! Using the template method allows a new class of payment failure:
//! user error (lock mismatch).
//!
//! If `final_cltv` is None, a server default is used.
//! We follow the naming and structure of the spec.

use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "/quote";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Body {
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub payee: [u8; 33],
    #[n(1)]
    pub amount_msat: u64,
    #[n(2)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub r_hash: [u8; 32],
    #[n(3)]
    /// If not included, assume default
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub final_cltv: Option<u64>,
    #[n(4)]
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Vec::is_empty"))]
    pub route_hints: Vec<RouteHint>,
    /// The payment secret is an anti-probing defense.
    /// Not to be confused with the r_preimage which in Konduit we refer to as `Secret`.
    #[n(5)]
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "serde_with::As::<Option<serde_with::hex::Hex>>"
        )
    )]
    pub payment_secret: Option<[u8; 32]>,
}

pub type Response = crate::auth::pay::common::quote::ChequeProposal;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::quote::Error;

/// We restate RouteHint with all derivations.
/// A route hint included in a BOLT11 invoice, describing a path of hops
/// the payer can use to reach the recipient.
///
/// An invoice may contain multiple [`RouteHint`]s, giving the payer
/// several candidate paths. Each hint is a sequence of [`RouteHintHop`]s
/// to traverse in order.
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cbor(transparent)]
pub struct RouteHint(#[n(0)] pub Vec<RouteHintHop>);

/// A single hop within a [`RouteHint`], describing a channel that can
/// be used to reach the recipient of a BOLT11 invoice.
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RouteHintHop {
    /// The node through which this hop routes.
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub src_node_id: [u8; 33],

    /// The channel to use for this hop, identified by its short channel ID.
    #[n(1)]
    pub short_channel_id: u64,

    /// Fees charged by this hop for forwarding an HTLC.
    #[n(2)]
    pub fees: RoutingFees,

    /// The CLTV delta added by this hop, in blocks.
    #[n(3)]
    pub cltv_expiry_delta: u16,

    /// The minimum HTLC size this hop will forward, in millisatoshis.
    #[n(4)]
    pub htlc_minimum_msat: Option<u64>,

    /// The maximum HTLC size this hop will forward, in millisatoshis.
    #[n(5)]
    pub htlc_maximum_msat: Option<u64>,
}

/// Fees charged by a hop when routing a payment.
///
/// Used within [`RouteHintHop`] to describe the cost of forwarding
/// an HTLC through a particular channel.
///
/// ```math
/// fee = base_msat + (amount_msat * proportional_millionths / 1_000_000)
/// ```
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RoutingFees {
    /// Flat fee, in millisatoshis, charged for forwarding any HTLC.
    #[n(0)]
    pub base_msat: u32,

    /// Proportional fee, in millionths of the HTLC amount,
    /// charged for forwarding. For example, `1000` means 0.1%.
    #[n(1)]
    pub proportional_millionths: u32,
}
