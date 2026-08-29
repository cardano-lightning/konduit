use std::cmp;

use konduit_data::Duration;
use web_time::{SystemTime, UNIX_EPOCH};

/// A  hack to solve an issue where Blockfrost would report
/// a tx as being from the future.
fn one_minite_ago() -> Duration {
    Duration::from_secs(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(60),
    )
}

#[derive(Debug, Clone, Default)]
pub struct Interval {
    pub lower: Option<Duration>,
    pub upper: Option<Duration>,
}

impl Interval {
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

    /// An interval n minites into the future.
    /// Note that the lower bound is set to one minite earlier than now.
    /// The intervale will be `n+1` minutes long!
    pub fn n_mins(n: u64) -> Self {
        let lower = one_minite_ago();
        let upper = Duration::from_secs(lower.as_secs() + n * 60);
        Interval {
            lower: Some(lower),
            upper: Some(upper),
        }
    }

    // TODO:: remove these
    pub fn five_mins() -> Self {
        Self::n_mins(5)
    }

    pub fn twenty_mins() -> Self {
        Self::n_mins(20)
    }
}
