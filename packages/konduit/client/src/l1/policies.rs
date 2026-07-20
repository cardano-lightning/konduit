use konduit_data::Duration;
use konduit_tx::Bounds;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum SubmitPolicy {
    #[n(0)]
    ViaConnector,
    #[n(1)]
    ViaWallet,
}

impl Default for SubmitPolicy {
    fn default() -> Self {
        SubmitPolicy::ViaConnector
    }
}

/// How far into the future the transaction's upper validity bound is set,
/// anchored to the moment `build` runs. Defaults to 20 minutes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub struct BoundsPolicy {
    #[n(0)]
    window: Duration,
}

impl BoundsPolicy {
    pub fn new(window: Duration) -> Self {
        Self { window }
    }
    pub fn window(&self) -> Duration {
        self.window
    }
    pub fn set_window(&mut self, window: Duration) {
        self.window = window;
    }

    pub(crate) fn to_bounds(self, start_at: &Duration) -> Bounds {
        Bounds::start_at(start_at, Some(&self.window))
    }
}

impl Default for BoundsPolicy {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(20 * 60),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Policies {
    #[n(0)]
    submit: SubmitPolicy,
    #[n(1)]
    bounds: BoundsPolicy,
    /// Autocomplete will expire, elapse, and end channels where possible.
    /// When `true`, every eligible channel is auto-elapsed/-expired/-ended
    /// and `Directives::force` is ignored. When `false`, only inputs in
    /// `force` are elapsed/expired/ended.
    /// FIXME: currently this flag is not consulted anywhere — every
    /// eligible channel is auto-elapsed/-expired/-ended unconditionally
    /// in `L1::build`, regardless of `autocomplete` or `force`.
    #[n(2)]
    autocomplete: bool,
}

impl Policies {
    pub fn submit(&self) -> &SubmitPolicy {
        &self.submit
    }
    pub fn set_submit(&mut self, policy: SubmitPolicy) {
        self.submit = policy;
    }
    pub fn bounds(&self) -> &BoundsPolicy {
        &self.bounds
    }
    pub fn set_bounds(&mut self, policy: BoundsPolicy) {
        self.bounds = policy;
    }
    pub fn autocomplete(&self) -> bool {
        self.autocomplete
    }
    pub fn set_autocomplete(&mut self, autocomplete: bool) {
        self.autocomplete = autocomplete;
    }
}
