use konduit_data::{Squash, SquashBody, Unlocked};
use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};

pub const ENDPOINT: &str = "/squash";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct Body(#[n(0)] pub Squash);

// Into
impl From<Body> for Squash {
    fn from(r: Body) -> Self {
        r.0
    }
}

// From
impl From<Squash> for Body {
    fn from(s: Squash) -> Self {
        Self(s)
    }
}

// Borrow
impl std::ops::Deref for Body {
    type Target = Squash;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Response {
    #[n(0)]
    Ok,
    #[n(1)]
    Stale(#[n(0)] SquashProposal),
}

pub type Error = super::Error<DomainError>;

#[derive(ProblemDetail)]
pub enum DomainError {
    /// Something is wrong with the data, for example invalid signature.
    #[problem(slug = "invalid-squash", title = "Invalid Squash", http_status = 400)]
    InvalidSquash,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct SquashProposal {
    #[n(0)]
    pub current: Squash,
    #[n(1)]
    pub unlockeds: Vec<Unlocked>,
    #[n(2)]
    pub proposal: SquashBody,
}
