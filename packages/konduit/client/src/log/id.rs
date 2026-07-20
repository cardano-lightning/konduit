use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use web_time::{SystemTime, UNIX_EPOCH};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Encode, Decode,
)]
#[repr(transparent)]
#[cbor(transparent)]
#[serde(transparent)]
pub struct Id(#[n(0)] u64);

impl Id {
    pub fn new(millis_since_epoch: u64) -> Self {
        Self(millis_since_epoch)
    }

    /// Lossy — display only, never fed back into the logger.
    pub fn as_time(&self) -> SystemTime {
        UNIX_EPOCH + web_time::Duration::from_millis(self.0)
    }

    fn now() -> Self {
        Self::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        )
    }
}
