use cardano_sdk::{Address, address::kind};
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::{BoundsPolicy, Policies, SubmitPolicy};
use crate::core::Credential;

/// Locally-authored configuration: set directly by the caller and left
/// alone until changed. Not recoverable from the chain if lost.
#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    /// User policies
    #[n(0)]
    policies: Policies,
    /// Reference script address ie where to find a utxo with the reference script.
    #[n(1)]
    reference_script_address: Option<Address<kind::Shelley>>,
    /// User delegation credentials. This accommodates the case where
    /// user has changed their delegation address mid channel lifecycles
    #[n(2)]
    delegations: Vec<Credential>,
    /// User preferred change address if different from wallet.
    #[n(3)]
    change_address: Option<Address<kind::Any>>,
}

impl Config {
    pub fn policies(&self) -> &Policies {
        &self.policies
    }
    pub fn submit_policy(&self) -> &SubmitPolicy {
        &self.policies().submit()
    }
    pub fn set_submit_policy(&mut self, policy: SubmitPolicy) {
        self.policies.set_submit(policy);
    }
    pub fn bounds_policy(&self) -> &BoundsPolicy {
        self.policies().bounds()
    }
    pub fn set_bounds_policy(&mut self, policy: BoundsPolicy) {
        self.policies.set_bounds(policy);
    }
    pub fn autocomplete(&self) -> bool {
        self.policies().autocomplete()
    }
    pub fn set_autocomplete(&mut self, autocomplete: bool) {
        self.policies.set_autocomplete(autocomplete);
    }

    pub fn reference_script_address(&self) -> Option<Address<kind::Shelley>> {
        self.reference_script_address.clone()
    }
    pub fn set_reference_script_address(&mut self, address: Address<kind::Shelley>) {
        self.reference_script_address = Some(address);
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
