use cardano_sdk::{Address, address::kind};
use konduit_data::Duration;
use konduit_tx::Bounds;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::core::Credential;

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
struct Policies {
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

/// Locally-authored configuration: set directly by the caller and left
/// alone until changed. Not recoverable from the chain if lost.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Config {
    #[n(0)]
    change_address: Option<Address<kind::Any>>,
    #[n(1)]
    delegations: Vec<Credential>,
    #[n(2)]
    policies: Policies,
}

impl Config {
    pub fn submit_policy(&self) -> SubmitPolicy {
        self.policies.submit
    }
    pub fn set_submit_policy(&mut self, policy: SubmitPolicy) {
        self.policies.submit = policy;
    }
    pub fn bounds_policy(&self) -> BoundsPolicy {
        self.policies.bounds
    }
    pub fn set_bounds_policy(&mut self, policy: BoundsPolicy) {
        self.policies.bounds = policy;
    }
    pub fn autocomplete(&self) -> bool {
        self.policies.autocomplete
    }
    pub fn set_autocomplete(&mut self, autocomplete: bool) {
        self.policies.autocomplete = autocomplete;
    }

    pub fn delegations(&self) -> Vec<Credential> {
        self.delegations.clone()
    }
    pub fn add_delegation(&mut self, credential: Credential) {
        self.delegations.push(credential);
    }
    pub fn remove_delegation(&mut self, credential: &Credential) {
        self.delegations.retain(|c| c != credential);
    }

    pub fn change_address(&self) -> Option<Address<kind::Any>> {
        self.change_address.clone()
    }
    pub fn set_change_address(&mut self, address: Address<kind::Any>) {
        self.change_address = Some(address);
    }
}
