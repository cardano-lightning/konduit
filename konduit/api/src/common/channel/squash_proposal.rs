use konduit_data::{Locked, Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// The server's proposal for the next squash the client should sign,
/// together with the in-flight cheques needed to reproduce it.
///
/// Returned in `sync::Response::squash_proposal`.
/// `None` until the client has submitted their first (null) squash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "squash-proposal"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SquashProposal {
    /// The squash body the client must sign.
    #[cbor(n(0), with = "konduit_data::cbor_with::plutus_data")]
    #[cfg_attr(feature = "cddl", cddl(ty = "squash-body"))]
    pub proposal: SquashBody,
    /// The current (last accepted) squash.
    #[cbor(n(1), with = "konduit_data::cbor_with::plutus_data")]
    #[cfg_attr(feature = "cddl", cddl(ty = "squash"))]
    pub current: Squash,
    /// Cheques that have been unlocked since the last squash.
    #[cbor(n(2), with = "konduit_data::cbor_with::vec_plutus_data")]
    #[cfg_attr(feature = "cddl", cddl(ty = "[* unlocked]"))]
    pub unlockeds: Vec<Unlocked>,
    /// Cheques that are currently locked (in-flight payments).
    #[cbor(n(3), with = "konduit_data::cbor_with::vec_plutus_data")]
    #[cfg_attr(feature = "cddl", cddl(ty = "[* locked]"))]
    pub lockeds: Vec<Locked>,
}

impl From<konduit_data::SquashProposal> for SquashProposal {
    fn from(sp: konduit_data::SquashProposal) -> Self {
        Self {
            proposal: sp.proposal,
            current: sp.current,
            unlockeds: sp.unlockeds,
            lockeds: sp.lockeds,
        }
    }
}
