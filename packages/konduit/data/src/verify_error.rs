/// Error type for cryptographic signature verification failures.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("invalid signature")]
    InvalidSignature,
}
