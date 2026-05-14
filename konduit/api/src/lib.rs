/// Implemented by all wire-level error types.
/// Status code is a plain `u16` so the trait has no framework dependency.
/// The actix integration converts this to `actix_web::http::StatusCode`.
pub trait ApiError {
    fn status_code(&self) -> u16;
}

/// Cross cutting concerns. Types that appear in more than one endpoint.
pub mod common;

/// Faithful to the API structure.
/// Any type defined within the crate,
/// and appearing in exactly one endpoint
/// is found in the corresponding module.
pub mod endpoints;

/// Auth
pub mod auth;

/// Re-export `DepthBucket` — client-facing wire type for backing depth classification.
pub use common::channel::DepthBucket;

/// Format-negotiated handler return type and supporting utilities.
/// Requires feature `actix`.
#[cfg(feature = "actix")]
pub mod actix;
