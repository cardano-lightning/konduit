/// Cross cutting concerns. Types that appear in more than one endpoint
pub mod common;

/// Faithful to the API structure.
pub mod endpoints;

/// Auth
pub mod auth;

/// FIXME :: Move this to own crate.
pub mod channel;
