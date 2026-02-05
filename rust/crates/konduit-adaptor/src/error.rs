#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(String),
    #[error("Fx error: {0}")]
    Fx(#[from] fx_client::Error),
    #[error("erialization/Deserialization error: {0}")]
    DbError(#[from] serde_json::Error),
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Invalid data in DB: {0}")]
    InvalidData(String),
    #[error("Task execution error: {0}")]
    TaskJoin(String),
}

/// Implement conversion from anyhow::Error to our local Error type.
/// This allows the '?' operator to work with library functions returning anyhow::Result.
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        // We map anyhow errors to our Config variant as a catch-all.
        Error::Config(err.to_string())
    }
}
