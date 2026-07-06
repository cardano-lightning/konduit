#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid database location: {0}")]
    InvalidLocation(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(String),
}

pub type Result<T> = std::result::Result<T, Error>;
