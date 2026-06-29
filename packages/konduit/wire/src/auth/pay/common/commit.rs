use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;
use serde::{Deserialize, Serialize};

use crate::auth::squash::SquashProposal;

/// The common response is a ChequeProposal
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Status {
    /// Commitment resolved. Secret present in squash proposal.
    #[n(0)]
    Resolved(#[n(0)] SquashProposal),
    /// Payment is pending aka inflight. It has neither succeeded nor failed.
    /// Client should poll for updates.
    #[n(1)]
    Pending,
}

#[derive(ProblemDetail)]
pub enum Error {
    /// An error occurred either before server commitment or commitment is unwound.
    /// Server, _should_ accept reuse of index.
    #[problem(delegate)]
    Uncommitted(UncommittedError),
    /// An error occurred after server commitment, and without graceful resolution ie unwinding.
    /// Associated funds are locked until timeout.
    #[problem(delegate)]
    Committed(CommittedError),
}

#[derive(ProblemDetail)]
pub enum UncommittedError {
    /// The quote on which the commit is based is stale
    #[problem(slug = "commit-stale", title = "Commit Stale", http_status = 400)]
    Stale,
    /// The quote does not match the commit
    #[problem(slug = "commit-mismatch", title = "Commit Mismatch", http_status = 400)]
    Mismatch,
    /// A route no longer exists.
    #[problem(slug = "no-route", title = "No Route", http_status = 400)]
    NoRoute,
    /// Something went wrong
    #[problem(slug = "precommit-other", title = "Precommit Other", http_status = 400)]
    Other,
}

#[derive(ProblemDetail)]
pub enum CommittedError {
    /// A failure occured on the route.
    #[problem(slug = "bad-route", title = "Bad Route", http_status = 400)]
    BadRoute,
    /// Something went wrong
    #[problem(
        slug = "postcommit-other",
        title = "Postcommit Other",
        http_status = 400
    )]
    Other,
}
