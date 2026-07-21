use konduit_data::Duration;
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Policies {
    #[n(0)]
    reg: RegPolicy,
    #[n(1)]
    squash: SquashPolicy,
}

impl Policies {
    pub fn new(reg: RegPolicy, squash: SquashPolicy) -> Self {
        Self { reg, squash }
    }
    pub fn reg(&self) -> &RegPolicy {
        &self.reg
    }
    pub fn set_reg(&mut self, policy: RegPolicy) {
        self.reg = policy;
    }
    pub fn squash(&self) -> &SquashPolicy {
        &self.squash
    }
    pub fn set_squash(&mut self, policy: SquashPolicy) {
        self.squash = policy;
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RegPolicy {
    #[n(0)]
    None,
    #[n(1)]
    Auth(#[n(0)] Duration),
}

impl Default for RegPolicy {
    fn default() -> Self {
        RegPolicy::Auth(Duration::from_secs(24 * 60 * 60))
    }
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SquashPolicy {
    /// No automatic squash handling. `sync`/`commit` verify at most one
    /// proposal and return without resubmitting — resolving further is
    /// left to the caller.
    #[n(0)]
    Manual,
    #[n(1)]
    Lenient {
        #[n(0)]
        retry: u8,
    },
    /// Verify signatures, and reject any unlocked whose expiry predates
    /// `last_received` — the last time this client actually received a
    /// squash proposal from the server. Guards against replaying a stale
    /// proposal as current. Clock drift is not a practical concern at the
    /// hours/minutes timescales relevant here.
    #[n(2)]
    RejectOld {
        #[n(0)]
        retry: u8,
    },
}

impl SquashPolicy {
    pub fn lenient(retry: u8) -> Self {
        SquashPolicy::Lenient { retry }
    }
    pub fn reject_old(retry: u8) -> Self {
        SquashPolicy::RejectOld { retry }
    }
}

/// `last_received` defaults to POSIX TIME 0 — indicates no squash
/// proposal ever received, so all unlockeds are treated as more recent.
impl Default for SquashPolicy {
    fn default() -> Self {
        SquashPolicy::reject_old(3)
    }
}
