/// Error type for parsing and conversion failures on primitive types.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("invalid integer: {0}")]
    Int(#[from] std::num::ParseIntError),

    #[error("expected {expected} bytes, got {got}")]
    WrongLength { expected: usize, got: usize },

    #[error("{0}")]
    Constraint(String),
}
