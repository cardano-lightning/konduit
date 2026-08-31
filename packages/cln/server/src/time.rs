use konduit_data::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

pub fn now() -> Duration {
    Duration::from_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed calculate duration since UNIX epoch ?!")
            .as_millis() as u64,
    )
}
