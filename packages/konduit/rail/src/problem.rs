//! What a rail is allowed to say went wrong.
//!
//! The enums carry no HTTP: the slug is theirs, the status is the
//! boundary's, and konduit applies the mapping as a problem leaves over
//! HTTP. Slugs are stable across rails. Runtime context travels as a
//! `String` for now; these become `problem-details` derives when that
//! crate is wired in.

/// Everything that blocks a quote, and every way a commit fails before
/// anything is sent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QuoteProblem {
    /// The instruction failed to deserialize. Raised by the erasing layer, so
    /// no rail hand rolls it.
    #[error("bad instruction: {0}")]
    BadInstruction(String),
    /// The instruction parsed, and there is no route to it.
    #[error("unroutable: {0}")]
    Unroutable(String),
    /// The amount is outside what the network carries.
    #[error("amount out of range: {0}")]
    AmountOutOfRange(String),
    /// The rail's own backend is unreachable.
    #[error("rail unavailable: {0}")]
    RailUnavailable(String),
    /// No rate for the instance asset. Placeholder: no rail fetches a
    /// rate yet, so nothing produces this.
    #[error("fx unavailable: {0}")]
    FxUnavailable(String),
}

impl QuoteProblem {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::BadInstruction(_) => "bad-instruction",
            Self::Unroutable(_) => "unroutable",
            Self::AmountOutOfRange(_) => "amount-out-of-range",
            Self::RailUnavailable(_) => "rail-unavailable",
            Self::FxUnavailable(_) => "fx-unavailable",
        }
    }
}

/// A `QuoteProblem`, or the constraints failing to cover what the rail's
/// own backend asks. Commit alone raises the latter, since quote is what
/// proposes the margin.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreProblem {
    #[error(transparent)]
    Quote(#[from] QuoteProblem),
    #[error("insufficient margin: {0}")]
    InsufficientMargin(String),
}

impl PreProblem {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Quote(problem) => problem.slug(),
            Self::InsufficientMargin(_) => "insufficient-margin",
        }
    }
}

/// Nothing fails before sending once sent, so these are the only two.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PostProblem {
    /// The outbound network refused or failed the payment.
    #[error("outbound rejected: {0}")]
    OutboundRejected(String),
    /// The outbound commitment timed out unresolved. Reveal only.
    #[error("outbound timeout: {0}")]
    OutboundTimeout(String),
}

impl PostProblem {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::OutboundRejected(_) => "outbound-rejected",
            Self::OutboundTimeout(_) => "outbound-timeout",
        }
    }
}
