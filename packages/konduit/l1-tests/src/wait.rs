use konduit_data::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub poll: Duration,
    pub max_attempts: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(10),
            max_attempts: 10,
        }
    }
}
