use konduit_data::{Cheque, Squash};
use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};

pub const ENDPOINT: &str = "/state";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

/// TODO:: Do we want query params?
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Params {}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
    /// This may be the case for numerous reasons.
    /// For example, the channel was closed or server no longer recognizes it.
    #[n(0)]
    pub backing: Backing,
    #[n(1)]
    pub receipt: Receipt,
}

pub type Error = super::Error<DomainError>;

#[derive(ProblemDetail)]
pub enum DomainError {
    /// FIXME :: Something went wrong.
    #[problem(slug = "state-other", title = "State Other", http_status = 400)]
    Other,
}

/// Backing is treated as purely informational.
/// This value may be cached; it may be stale at the
/// point at which the user requests a quote
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Backing {
    /// Amount that server deems confirmed on-chain,
    /// and is backing the channel.
    /// None indicates channel is not available.
    #[n(0)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settled: Option<Amounts>,
    /// Amount that is seen on-chain but not yet settled.
    /// This can alleviate some UX issues
    #[n(1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<Amounts>,
}

/// TODO:: More explicit term desired, but this is literally "amounts".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Amounts {
    /// (Total) subbed according to the datum
    #[n(0)]
    pub subbed: u64,
    /// Amount of asset in utxo
    #[n(1)]
    pub balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Receipt {
    #[n(0)]
    pub squash: Squash,
    #[n(1)]
    pub cheques: Vec<Cheque>,
}
