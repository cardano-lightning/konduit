//! Authorized endpoint.
//!
//! User must have registered `./reg`.
//! They then use the (auth) scheme credential.
//!
//! ## Header
//!
//! Header is of the form `Authorization: <scheme> <credential>`.
//! Credentials are base64 encoded.

use problem_details::ProblemDetail;

use crate::limit::LimitError;
pub mod pay;
pub mod squash;
pub mod state;

const ENDPOINT: &str = "/auth";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

// All child routes may use this error.
#[derive(ProblemDetail)]
pub enum Error<T> {
    #[problem(delegate)]
    Auth(AuthError),
    #[problem(delegate)]
    Limit(LimitError),
    #[problem(delegate)]
    Domain(T),
}

#[derive(ProblemDetail)]
pub enum AuthError {
    /// Credential required, none found or could not be sensibly coerced.
    #[problem(slug = "no-credential", title = "No Credential", http_status = 400)]
    None,
    /// Credential invalid. Try registration
    #[problem(
        slug = "invalid-credential",
        title = "Invalid Credential",
        http_status = 400
    )]
    Invalid,
}
