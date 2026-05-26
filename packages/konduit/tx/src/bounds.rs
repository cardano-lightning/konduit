use std::cmp;

use konduit_data::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct Bounds {
    pub lower: Option<Duration>,
    pub upper: Option<Duration>,
}

impl Bounds {
    pub fn lower(lower: Duration) -> Self {
        Self {
            lower: Some(lower),
            upper: None,
        }
    }

    pub fn upper(upper: Duration) -> Self {
        Self {
            lower: None,
            upper: Some(upper),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let lower = match (self.lower, other.lower) {
            (Some(a), Some(b)) => Some(cmp::max(a, b)),
            (a, b) => a.or(b),
        };
        let upper = match (self.upper, other.upper) {
            (Some(a), Some(b)) => Some(cmp::min(a, b)),
            (a, b) => a.or(b),
        };

        Self { lower, upper }
    }

    pub fn twenty_mins() -> Self {
        // TODO :: Either use std time, or upstream methods
        let lower = Duration::from_secs(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                // Hack to handle blockfrost slots not aligning with current time.
                .saturating_sub(60),
        );
        let upper = Duration::from_secs(lower.as_secs() + 19 * 60);
        Bounds {
            lower: Some(lower),
            upper: Some(upper),
        }
    }
}
