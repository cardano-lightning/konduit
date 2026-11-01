//! What a rail is allowed to say went wrong.
//!
//! Slug and status both belong here, with the enum that defines the
//! problem: they are stable across rails and across transports, and the
//! variant's author is who knows what it means. Status is a `u16`, not
//! an `http::StatusCode` — transport-agnostic vocabulary, per
//! [ADR 10](../../../../docs/adrs/10-problem-details.md) — so a boundary
//! turns it into a response and chooses nothing. Runtime context travels
//! as a `String` for now; these become `problem-details` derives, whose
//! trait carries the same two, when that crate is wired in.

use serde::{Deserialize, Serialize};

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

    pub fn status(&self) -> u16 {
        match self {
            // The instruction is malformed, so the request is.
            Self::BadInstruction(_) => 400,
            // Well formed, and cannot be acted on.
            Self::Unroutable(_) | Self::AmountOutOfRange(_) => 422,
            // Nothing wrong with the asking; try later.
            Self::RailUnavailable(_) | Self::FxUnavailable(_) => 503,
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

    pub fn status(&self) -> u16 {
        match self {
            Self::Quote(problem) => problem.status(),
            Self::InsufficientMargin(_) => 422,
        }
    }
}

/// Nothing fails before sending once sent, so these are the only two.
///
/// Serializable because a rail that persists what it has committed
/// persists how those commitments ended.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
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

    pub fn status(&self) -> u16 {
        match self {
            // The failure is downstream of konduit, and konduit is the
            // gateway to it.
            Self::OutboundRejected(_) => 502,
            Self::OutboundTimeout(_) => 504,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slugs and statuses are the published contract, so they are
    /// worth asserting rather than reading.
    #[test]
    fn the_published_contract() {
        let detail = String::new();
        for (problem, slug, status) in [
            (
                QuoteProblem::BadInstruction(detail.clone()),
                "bad-instruction",
                400,
            ),
            (QuoteProblem::Unroutable(detail.clone()), "unroutable", 422),
            (
                QuoteProblem::AmountOutOfRange(detail.clone()),
                "amount-out-of-range",
                422,
            ),
            (
                QuoteProblem::RailUnavailable(detail.clone()),
                "rail-unavailable",
                503,
            ),
            (
                QuoteProblem::FxUnavailable(detail.clone()),
                "fx-unavailable",
                503,
            ),
        ] {
            assert_eq!((problem.slug(), problem.status()), (slug, status));
        }

        let margin = PreProblem::InsufficientMargin(detail.clone());
        assert_eq!(
            (margin.slug(), margin.status()),
            ("insufficient-margin", 422)
        );
        // A quote problem keeps its own slug and status when a commit
        // raises it before sending.
        let delegated = PreProblem::from(QuoteProblem::Unroutable(detail.clone()));
        assert_eq!((delegated.slug(), delegated.status()), ("unroutable", 422));

        for (problem, slug, status) in [
            (
                PostProblem::OutboundRejected(detail.clone()),
                "outbound-rejected",
                502,
            ),
            (
                PostProblem::OutboundTimeout(detail),
                "outbound-timeout",
                504,
            ),
        ] {
            assert_eq!((problem.slug(), problem.status()), (slug, status));
        }
    }
}
