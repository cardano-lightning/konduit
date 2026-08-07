use std::time::{SystemTime, UNIX_EPOCH};

use konduit_data::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no system time")]
    NoTime,
}

pub fn now() -> Result<konduit_data::Duration, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(to_konduit_duration)
        .map_err(|_| Error::NoTime)
}

pub fn to_konduit_duration(x: std::time::Duration) -> konduit_data::Duration {
    Duration::from_millis(x.as_millis() as u64)
}

pub fn from_konduit_duration(x: konduit_data::Duration) -> std::time::Duration {
    std::time::Duration::from_millis(x.as_millis() as u64)
}
