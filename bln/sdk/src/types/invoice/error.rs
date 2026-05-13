#[derive(Debug, Clone, thiserror::Error)]
pub enum InvoiceError {
    #[error("Parse Error")]
    Parse,
    #[error("Bad input")]
    BadInput,
    #[error("Cannot handle picosatoshi")]
    AmountPico,
    #[error("Amount Overflow")]
    AmountOverflow,
    #[error("Missing field {0}")]
    MissingField(String),
}
