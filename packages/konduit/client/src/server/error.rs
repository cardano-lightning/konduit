use problem_details::ProblemDetailBody;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no credential set: send set_credential first")]
    MissingCredential,

    #[error("transport error: {0}")]
    Transport(Box<dyn std::error::Error + Send + Sync>),

    /// Encoder/Decoder errors are funneled through `Codec::Error` first —
    /// a real, meaningful, per-codec type (e.g. `CborCodecError`'s named
    /// Encode/Decode variants) — and only then boxed here. This is the
    /// "honest conversion point": what's boxed is what you'd actually
    /// want back if you downcast it, not a raw, uncategorized leaf.
    #[error("codec error: {0}")]
    Codec(Box<dyn std::error::Error + Send + Sync>),

    #[error("HTTP construction error: {0}")]
    Http(#[from] http::Error),

    #[error("client error {status}: {problem:?}")]
    Problem {
        status: http::StatusCode,
        problem: ProblemDetailBody,
    },
    #[error("client error {status}: body did not match expected Problem Details shape")]
    ProblemUnparsed {
        status: http::StatusCode,
        raw_body: Vec<u8>,
    },
    #[error("server error {status}: unhandled")]
    ServerStatus {
        status: http::StatusCode,
        raw_body: Vec<u8>,
    },
}
