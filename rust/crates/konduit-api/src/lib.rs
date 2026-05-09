/// Cross cutting concerns. Types that appear in more than one endpoint
pub mod common;

/// Faithful to the API structure.
/// Any type defined within the crate, 
/// and appearing in exactly one endpont 
/// is found in the corresponding module
pub mod endpoints;

/// Auth
pub mod auth;

// FIXME :: Move this to own crate.
pub mod channel;

mod local_cbor_with;
