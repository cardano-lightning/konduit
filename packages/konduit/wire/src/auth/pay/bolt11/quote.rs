//! BOLT-11 aka invoice.

const ENDPOINT: &str = "/quote";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

/// Body is just invoice: the bech32 encoding string as it appears to the user. No funny business.
pub type Body = String;

pub type Response = crate::auth::pay::common::quote::ChequeProposal;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::quote::Error;
