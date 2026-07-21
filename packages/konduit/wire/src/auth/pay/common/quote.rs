use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The common response is a ChequeProposal
#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChequeProposal {
    /// Cheque index
    #[n(0)]
    pub index: u64,
    /// Cheque amount
    #[n(1)]
    pub amount: u64,
    /// Cheque timeout. Note that this is **relative** and in ms
    #[n(2)]
    pub relative_timeout: u64,
    /// Routing fee. This is informational - the import value is the `amount`.
    /// Clients should independently calculate the effective fee.
    #[n(3)]
    pub fee: u64,
}

#[derive(ProblemDetail)]
pub enum Error {
    /// There no backing.
    /// This may be because none exists on-chain or server is not serving this channel
    #[problem(slug = "no-backing", title = "No Backing", http_status = 400)]
    Backing,

    /// The channel has no squash.
    #[problem(slug = "no-squash", title = "No squash", http_status = 400)]
    Squash,

    /// Channel has no room for more cheques: too many unresolved payments or in-flight cheques.
    /// Client must submit a squash to free capacity before retrying,
    /// or may have to wait until in-flight cheques timeout.
    #[problem(slug = "no-capacity", title = "No Capacity", http_status = 400)]
    Capacity,

    /// The backing has insufficient funds
    #[problem(
        slug = "insufficient-funds",
        title = "Insufficient Funds",
        http_status = 400
    )]
    Funds,

    /// No route found
    #[problem(slug = "no-route", title = "No Route", http_status = 400)]
    Route,

    /// Request payload exceeds the maximum size.
    #[problem(slug = "max-size", title = "Max Size", http_status = 400)]
    Size,
}
