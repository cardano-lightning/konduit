use std::time::SystemTimeError;

use konduit_data::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("System time error")]
    System,
}

impl From<SystemTimeError> for Error {
    fn from(_value: SystemTimeError) -> Self {
        Error::System
    }
}

pub fn now() -> Result<Duration, Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(Duration::from_millis(now.as_millis() as u64))
}
