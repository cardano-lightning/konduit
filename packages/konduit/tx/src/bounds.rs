use std::cmp;

use konduit_data::Duration;
use minicbor::{Decode, Encode};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Bounds {
    #[n(0)]
    pub lower: Option<Duration>,
    #[n(1)]
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

    /// Get a bounded window from (about) now.
    /// "about" to handle staleness from external services
    pub fn start_at(start: &Duration, window: Option<&Duration>) -> Self {
        // Hack to handle blockfrost slots not aligning with current time.
        let buffer = Duration::from_secs(60);
        let lower = start.saturating_sub(buffer);
        let upper = window.map(|x| lower.saturating_add(*x));
        Bounds {
            lower: Some(lower),
            upper: upper,
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
}
