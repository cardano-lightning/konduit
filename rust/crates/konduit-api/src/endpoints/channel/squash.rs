//! `POST /channel/squash`
//!
//! The client submits a signed squash to advance the channel's squash state.
//!
//! The squash body must match the `squash_proposal.proposal` returned by
//! `GET /channel/sync`. The client signs that body and posts it here.
//!
//! The first submission must be a null squash (`amount=0, index=0, exclude=[]`).
//! The server will not accept `pay` requests until a squash has been submitted.

use crate::auth;
use konduit_data::Squash;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "squash-request", inner = "squash"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Request {
    #[cbor(n(0), with = "konduit_data::cbor_with::plutus_data")]
    pub squash: Squash,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "squash-response"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Response {
    #[n(0)]
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, thiserror::Error)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "squash-error"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Error {
    #[n(0)]
    #[error(transparent)]
    Auth(#[n(0)] auth::pop::Error),

    #[n(1)]
    #[error("rate limit exceeded: {0}")]
    Limit(#[n(0)] String),

    #[n(2)]
    #[error("no backing")]
    Backing,

    /// Squash body doesn't match the current proposal, or signature doesn't verify.
    #[n(3)]
    #[error("bad squash")]
    Squash,

    /// The proposal has advanced (new cheques arrived) since the client last synced.
    /// The client must re-sync before retrying.
    #[n(4)]
    #[error("stale: re-sync required")]
    Stale,
}

impl crate::ApiError for Error {
    fn status_code(&self) -> u16 {
        match self {
            Self::Auth(e) => e.status_code(),
            Self::Limit(_) => 429,
            Self::Backing => 404,
            Self::Squash => 422,
            Self::Stale => 409,
        }
    }
}
