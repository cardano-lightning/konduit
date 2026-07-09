use konduit_data::Locked;

pub const ENDPOINT: &str = "/commit";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

pub type Body = Locked;

pub type Response = crate::auth::pay::common::commit::Status;

pub type Error = crate::auth::Error<DomainError>;

pub type DomainError = crate::auth::pay::common::commit::Error;
