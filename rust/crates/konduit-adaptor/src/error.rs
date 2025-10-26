#[derive(Debug, Error)]
pub enum AppError {
    #[error("Config error: {0}")]
    Config(#[from] sled::Error),
    #[error("erialization/Deserialization error: {0}")]
    DbError(#[from] serde_json::Error),
    #[error("Item not found: {0}")]
    NotFound(String),
    #[error("Invalid data in DB: {0}")]
    InvalidData(String),
    #[error("Task execution error: {0}")]
    TaskJoin(String),
}
