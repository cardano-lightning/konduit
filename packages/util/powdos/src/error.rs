//! Error type for proof-of-work operations.

use thiserror::Error;

/// Errors returned by [`Challenge::new`], [`Challenge::verify`], and [`Challenge::solve`].
#[cfg_attr(
    feature = "problem-details",
    derive(problem_details_wire::ProblemDetail)
)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// The challenge window has closed. Request a new challenge.
    #[error("challenge has expired")]
    #[cfg_attr(
        feature = "problem-details",
        problem(slug = "expired", title = "Challenge Expired", http_status = 400)
    )]
    Time,

    /// The challenge was not issued for this client or has been tampered with.
    #[error("challenge mac is invalid")]
    #[cfg_attr(
        feature = "problem-details",
        problem(
            slug = "invalid-mac",
            title = "Invalid Challenge MAC",
            http_status = 403
        )
    )]
    Mac,

    /// The submitted nonce does not meet the required difficulty.
    #[error("proof of work not satisfied")]
    #[cfg_attr(
        feature = "problem-details",
        problem(
            slug = "pow-failed",
            title = "Proof of Work Not Satisfied",
            http_status = 422
        )
    )]
    Hash,

    /// Enable `sha2` or `cryptoxide` (or both) in your `Cargo.toml`.
    #[error("unsupported proof-of-work scheme")]
    #[cfg_attr(
        feature = "problem-details",
        problem(
            slug = "unsupported-scheme",
            title = "Unsupported Proof-of-Work Scheme",
            http_status = 501
        )
    )]
    Scheme,
}
