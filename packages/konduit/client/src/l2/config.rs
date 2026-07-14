use cardano_sdk::Input;
use konduit_data::{Duration, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Data that changes rarely — explicit user action only (tag rotation,
/// policy changes). Safe to lose `policies` (defaults exist); losing `tag`
/// is not recoverable, since it's the identity every signature verifies
/// against.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    policies: Policies,
    #[n(1)]
    tag: Tag,
    #[n(2)]
    focus: Option<Input>,
}

impl Config {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            policies: Policies::default(),
            focus: None,
        }
    }

    pub fn tag(&self) -> Tag {
        self.tag.clone()
    }
    pub fn set_tag(&mut self, tag: Tag) {
        self.tag = tag;
    }

    pub fn policies(&self) -> &Policies {
        &self.policies
    }
    pub fn reg_policy(&self) -> RegPolicy {
        self.policies.reg()
    }
    pub fn set_reg_policy(&mut self, policy: RegPolicy) {
        self.policies.set_reg(policy);
    }
    pub fn squash_policy(&self) -> SquashPolicy {
        self.policies.squash()
    }
    pub fn set_squash_policy(&mut self, policy: SquashPolicy) {
        self.policies.set_squash(policy);
    }

    pub fn focus(&self) -> Option<&Input> {
        self.focus.as_ref()
    }
    pub fn set_focus(&mut self, focus: Option<Input>) {
        self.focus = focus;
    }
}

// ---------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, Default)]
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
    pub fn reg(&self) -> RegPolicy {
        self.reg
    }
    pub fn set_reg(&mut self, policy: RegPolicy) {
        self.reg = policy;
    }
    pub fn squash(&self) -> SquashPolicy {
        self.squash
    }
    pub fn set_squash(&mut self, policy: SquashPolicy) {
        self.squash = policy;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
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
        #[n(1)]
        last_received: Duration,
    },
}

impl SquashPolicy {
    pub fn lenient(retry: u8) -> Self {
        SquashPolicy::Lenient { retry }
    }

    pub fn reject_old(retry: u8, last_received: Duration) -> Self {
        SquashPolicy::RejectOld {
            retry,
            last_received,
        }
    }

    pub fn update_last_received(self, last_received: Duration) -> Self {
        let Self::RejectOld { retry, .. } = self else {
            return self;
        };
        Self::RejectOld {
            retry,
            last_received,
        }
    }
}

/// `last_received` defaults to POSIX TIME 0 — indicates no squash
/// proposal ever received, so all unlockeds are treated as more recent.
impl Default for SquashPolicy {
    fn default() -> Self {
        SquashPolicy::reject_old(3, Duration::from_millis(0))
    }
}
