use std::time::Duration;

use cardano_sdk::Hash;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Poll interval and attempt budget for a retry loop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    pub interval: Duration,
    pub max_attempts: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(5),
            max_attempts: 12,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("timed out waiting for transaction {id:?} after {attempts} attempts")]
    TimedOut { id: Hash<32>, attempts: u32 },
}

/// Drives poll/retry timing. Doesn't know what's being checked - callers
/// loop over `max_attempts` themselves and call `wait()` between tries.
#[derive(Debug, Clone, Copy)]
pub struct Waiter {
    config: Config,
}

impl Waiter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn max_attempts(&self) -> u32 {
        self.config.max_attempts
    }

    pub async fn wait(&self) {
        tokio::time::sleep(self.config.interval).await;
    }

    pub fn timed_out(&self, id: &Hash<32>) -> Error {
        Error::TimedOut {
            id: *id,
            attempts: self.config.max_attempts,
        }
    }
}
