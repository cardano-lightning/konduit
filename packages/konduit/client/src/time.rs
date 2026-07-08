use web_time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("failed to calculate duration since UNIX epoch ?!")
        .as_millis() as u64
}

pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Milliseconds since UNIX epoch, `offset` in the future.
pub fn now_plus(offset: Duration) -> u64 {
    (SystemTime::now() + offset)
        .duration_since(UNIX_EPOCH)
        .expect("failed to calculate duration since UNIX epoch ?!")
        .as_millis() as u64
}

pub fn now_plus_ms(offset: u64) -> u64 {
    now_plus(Duration::from_millis(offset))
}
