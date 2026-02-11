use konduit_data::Keytag;

use crate::ChannelError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Errors from a failure of the backend eg
    // connector breaks, memory full, serde errors.
    #[error("BackendError : {0}")]
    Backend(BackendError),
    // Logic failure (independent of how and where things are stored.)
    #[error("Logic : {0}")]
    Logic(LogicError),
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Serde : {0}")]
    Serde(String),
    #[error("Other : {0}")]
    Other(String),
}

impl From<BackendError> for Error {
    fn from(value: BackendError) -> Self {
        Self::Backend(value)
    }
}

impl From<postcard::Error> for BackendError {
    fn from(e: postcard::Error) -> Self {
        Self::Serde(e.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LogicError {
    // Expected an entry but none found.
    // For example trying to update squash when channel exists.
    #[error("NoEntry : {0}")]
    NoEntry(Keytag),
    // Channel failure
    #[error("Channel : {0}")]
    Channel(ChannelError),
}

impl From<ChannelError> for LogicError {
    fn from(value: ChannelError) -> Self {
        LogicError::Channel(value)
    }
}

impl From<ChannelError> for Error {
    fn from(value: ChannelError) -> Self {
        Self::Logic(LogicError::Channel(value))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
