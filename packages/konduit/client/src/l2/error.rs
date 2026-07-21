use crate::{commitments, server, time};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("time: {0}")]
    Time(#[from] time::Error),
    #[error("commitments: {0}")]
    Commitments(#[from] commitments::Error),
    #[error("no credential set: call reg first")]
    MissingCredential,
    #[error("credential expired: call reg again")]
    CredentialExpired,
    #[error("no pending payment request: must quote before commit")]
    PayRequestMissing,
    #[error("failed to parse pending payment request")]
    PayRequestCorrupt,
    #[error(transparent)]
    Squash(#[from] SquashError),
    #[error(transparent)]
    Server(#[from] server::Error),
    // #[error(transparent)]
    // State(#[from] state::Error),
    #[error("Signing error {0}")]
    Signing(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SquashError {
    #[error("current squash does not verify against our signing key")]
    CurrentInvalid,
    #[error("unlocked entry does not verify against our signing key")]
    UnlockedInvalid,
    #[error("unlocked entry predates last_received cutoff; rejecting as stale")]
    UnlockedOld,
    #[error("failed to merge verified unlocked into calculated squash: {0}")]
    Unlocked(String),
    #[error(
        "server proposal is not provably covered by calculated squash (proposed must be <= calculated under the partial order)"
    )]
    OverProposed,
    #[error("Calculated squash is incompatible with proposed")]
    Calculated,
    #[error("exhausted retry policy without reaching a complete squash")]
    RetriesExhausted,
    #[error("resolved squash proposal did not include the secret for this payment")]
    MissingPaymentSecret,
}
