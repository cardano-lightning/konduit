// state.rs — was the top-level `Cache`
use cardano_sdk::{
    Address, Output, Transaction, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::Tag;
use konduit_tx::{Channel, NetworkParameters, Utxo};
use std::collections::BTreeMap;

use crate::core::{Credential, Input};

use super::Cache;
use super::{BoundsPolicy, Config, SubmitPolicy};
use super::{Directives, Intent, OpenIntent};

/// Full state, ordered slowest to fastest: [`Config`] (user-authored,
/// not recoverable), [`Cache`] (chain-pulled, fully disposable), then
/// [`Directives`] (pending build intent, not recoverable but doesn't
/// carry `Config`'s "set once and forget" cadence either).
///
/// Staleness is the caller's responsibility: call `L1::pull_reference_script`,
/// `L1::pull_network_parameters`, and `L1::pull_channels` (or `L1::pull_all`)
/// as often as your staleness tolerance requires.
#[derive(
    Debug, Clone, Default, minicbor::Encode, minicbor::Decode, serde::Serialize, serde::Deserialize,
)]
pub struct State {
    #[n(0)]
    config: Config,
    #[n(1)]
    cache: Cache,
    #[n(2)]
    directives: Directives,
}

/// Everything `L1::build` needs, gathered in a single read.
pub(crate) struct BuildInputs {
    pub network_parameters: Option<NetworkParameters>,
    pub reference_script: Option<Utxo>,
    pub change_address: Option<Address<kind::Any>>,
    pub wallet_utxos: BTreeMap<Input, Output>,
    pub channels: BTreeMap<Input, (Output, Channel)>,
    pub opens: BTreeMap<Tag, OpenIntent>,
    pub intents: BTreeMap<Input, Intent>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Config passthroughs --
    pub fn submit_policy(&self) -> SubmitPolicy {
        self.config.submit_policy()
    }
    pub fn set_submit_policy(&mut self, policy: SubmitPolicy) {
        self.config.set_submit_policy(policy);
    }
    pub fn bounds_policy(&self) -> BoundsPolicy {
        self.config.bounds_policy()
    }
    pub fn set_bounds_policy(&mut self, policy: BoundsPolicy) {
        self.config.set_bounds_policy(policy);
    }
    pub fn autocomplete(&self) -> bool {
        self.config.autocomplete()
    }
    pub fn set_autocomplete(&mut self, autocomplete: bool) {
        self.config.set_autocomplete(autocomplete);
    }
    pub fn delegations(&self) -> Vec<Credential> {
        self.config.delegations()
    }
    pub fn add_delegation(&mut self, credential: Credential) {
        self.config.add_delegation(credential);
    }
    pub fn remove_delegation(&mut self, credential: &Credential) {
        self.config.remove_delegation(credential);
    }
    pub fn change_address(&self) -> Option<Address<kind::Any>> {
        self.config.change_address()
    }
    pub fn set_change_address(&mut self, address: Address<kind::Any>) {
        self.config.set_change_address(address);
    }

    // -- Cache passthroughs --
    pub fn network_parameters(&self) -> Option<NetworkParameters> {
        self.cache.network_parameters()
    }
    pub fn set_network_parameters(&mut self, network_parameters: NetworkParameters) {
        self.cache.set_network_parameters(network_parameters);
    }
    pub fn reference_script(&self) -> Option<Utxo> {
        self.cache.reference_script()
    }
    pub fn set_reference_script(&mut self, reference_script: Utxo) {
        self.cache.set_reference_script(reference_script);
    }
    pub fn channels(&self) -> BTreeMap<Input, (Output, Channel)> {
        self.cache.channels()
    }
    pub fn wallet_utxos(&self) -> BTreeMap<Input, Output> {
        self.cache.wallet_utxos()
    }

    /// Replace the cached tip, and drop any directive referring to an
    /// input that no longer has a live channel — the one place `Cache`
    /// and `Directives` need to be reconciled together.
    pub fn set_tip(
        &mut self,
        wallet_utxos: BTreeMap<Input, Output>,
        channels: BTreeMap<Input, (Output, Channel)>,
    ) {
        let live_inputs = self.cache.set_tip(wallet_utxos, channels);
        self.directives.retain_live(&live_inputs);
    }

    pub fn built_tx(&self) -> Option<Transaction<ReadyForSigning>> {
        self.cache.built_tx()
    }
    pub fn set_built_tx(&mut self, tx: Transaction<ReadyForSigning>) {
        self.cache.set_built_tx(tx);
    }
    pub fn clear_built_tx(&mut self) {
        self.cache.clear_built_tx();
    }

    // -- Directives passthroughs --
    pub fn intent(&self, input: &Input) -> Option<Intent> {
        self.directives.intent(input)
    }
    pub fn intents(&self) -> BTreeMap<Input, Intent> {
        self.directives.intents()
    }
    pub fn add_intent(&mut self, input: Input, intent: Intent) {
        self.directives.add_intent(input, intent);
    }

    /// Resolve `tag` against currently cached channels and set `intent`
    /// for the matching input(s). Returns the number of channels
    /// matched, so the caller can warn on ambiguous tags — reusing a tag
    /// across multiple channels is strongly discouraged but not an error
    /// at this layer.
    pub fn add_intent_by_tag(&mut self, tag: &Tag, intent: Intent) -> usize {
        let inputs: Vec<Input> = self
            .cache
            .channels()
            .iter()
            .filter(|(_, (_, channel))| channel.constants().tag == *tag)
            .map(|(input, _)| input.clone())
            .collect();
        let matched = inputs.len();
        for input in inputs {
            self.directives.add_intent(input, intent.clone());
        }
        matched
    }

    pub fn remove_intent(&mut self, input: &Input) {
        self.directives.remove_intent(input);
    }
    pub fn clear_intents(&mut self) {
        self.directives.clear_intents();
    }

    pub fn force(&self) -> std::collections::BTreeSet<Input> {
        self.directives.force()
    }
    pub fn add_force(&mut self, input: Input) {
        self.directives.add_force(input);
    }
    pub fn remove_force(&mut self, input: &Input) {
        self.directives.remove_force(input);
    }
    pub fn clear_force(&mut self) {
        self.directives.clear_force();
    }

    pub fn opens(&self) -> BTreeMap<Tag, OpenIntent> {
        self.directives.opens()
    }
    pub fn add_open(&mut self, open: OpenIntent) {
        self.directives.add_open(open);
    }
    pub fn remove_open(&mut self, tag: &Tag) {
        self.directives.remove_open(tag);
    }
    pub fn clear_opens(&mut self) {
        self.directives.clear_opens();
    }

    // -- Build snapshot --
    pub(crate) fn build_inputs(&self) -> BuildInputs {
        BuildInputs {
            network_parameters: self.cache.network_parameters(),
            reference_script: self.cache.reference_script(),
            change_address: self.config.change_address(),
            wallet_utxos: self.cache.wallet_utxos(),
            channels: self.cache.channels(),
            opens: self.directives.opens(),
            intents: self.directives.intents(),
        }
    }
}
