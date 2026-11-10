use std::time::{SystemTime, UNIX_EPOCH};

use konduit_core::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no system time")]
    NoTime,
}

pub fn now() -> Result<konduit_core::Duration, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(to_konduit_duration)
        .map_err(|_| Error::NoTime)
}

pub fn to_konduit_duration(x: std::time::Duration) -> konduit_core::Duration {
    Duration::from_millis(x.as_millis() as u64)
}
