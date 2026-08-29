use std::time::{SystemTime, UNIX_EPOCH};

use konduit_data::Duration;

pub fn now() -> Duration {
    Duration::from_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64,
    )
}
