#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Squash cannot include a (locked) cheque.")]
    IncludesCheque,

    #[error("Squash body was not reproduced")]
    NotReproduced,

    #[error("Bad input")]
    Input,

    #[error("Expected a change, but none observed")]
    Unchanged,

    #[error("Other")]
    Other,
}

// Convenient type alias for this module
pub type Result<T> = std::result::Result<T, Error>;
