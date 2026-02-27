use chrono::{DateTime, Local, Utc};
use web_time::{SystemTime, UNIX_EPOCH};

// POSIX seconds since the UNIX epoch useful because of slot rounding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PosixSeconds(pub u64);

pub type Slot = u64;

impl PosixSeconds {
    /// Get the current POSIX seconds.
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch");
        Self(duration.as_secs())
    }

    pub fn to_slot(&self, slot_config: &uplc::tx::SlotConfig) -> Slot {
        assert!(
            self.0 >= slot_config.zero_time,
            "Time is before the zero slot time"
        );
        let slot_length_secs = slot_config.slot_length / 1000;
        (self.0 - slot_config.zero_time) / (slot_length_secs as u64)
    }
}

impl From<DateTime<Utc>> for PosixSeconds {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt.timestamp() as u64)
    }
}

impl From<DateTime<Local>> for PosixSeconds {
    fn from(dt: DateTime<Local>) -> Self {
        let utc_dt: DateTime<Utc> = dt.with_timezone(&Utc);
        Self(utc_dt.timestamp() as u64)
    }
}
