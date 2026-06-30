//! # Registration
//!
//! There may be different _schemes_ for registration.
//! Regardless of the scheme, registration is required to send the initial squash.
//! Re-registration maybe required on, for example, an expiration of a auth token.
//!
//! An instance should support only one scheme.
//!
//! A scheme must specify:
//! - request body token
//! - an error type
//! - scheme name as appears in header (follow the existing conventions of RFC 7235)
//! - credential as appears in header. Should be base64 encoded.

use konduit_data::Squash;
use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};

use crate::limit::LimitError;

pub mod cobbl3;
pub mod no_auth;

const ENDPOINT: &str = "/reg";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

/// Request body
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Body<T> {
    /// Token depends on the authorization scheme used
    #[n(0)]
    pub token: T,
    /// The initial handshake must include a squash
    #[n(1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub squash: Option<Squash>,
}

#[derive(Debug, Clone, ProblemDetail)]
pub enum CommonError {
    #[problem(delegate)]
    Limit(LimitError),
    /// This can occur if the server has not seen the channel on-chain,
    /// or has removed the channel from the db. In either case,
    /// the server is not treating the channel as live.
    #[problem(slug = "no-channel", title = "No Channel", http_status = 400)]
    NoChannel,
    /// Bad input, for example invalid signature
    #[problem(slug = "bad-input", title = "Bad input", http_status = 400)]
    Input,
    /// Squash required. Particularly on initial registration
    #[problem(slug = "no-channel", title = "No Channel", http_status = 400)]
    Squash,
}
