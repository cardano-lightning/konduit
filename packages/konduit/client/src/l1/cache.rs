use cardano_sdk::{Output, Transaction, transaction::state::ReadyForSigning};
use konduit_tx::{Channel, NetworkParameters, Utxo};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::core::Input;

/// Pulled-from-chain state plus anything rebuildable from it: protocol
/// facts, current wallet/channel state, and the last built (unsigned)
/// transaction.
/// Fully disposable re-pull or rebuild replaces anything lost here.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct Cache {
    #[n(0)]
    network_parameters: Option<NetworkParameters>,
    #[n(1)]
    reference_script: Option<(Input, Output)>,
    #[n(2)]
    tip: Tip,
    #[n(3)]
    built_tx: Option<Transaction<ReadyForSigning>>,
}

/// Current on-chain state relevant to this wallet, keyed by input so
/// membership/lookup is O(log n) and there's no possibility of two
/// entries for the same utxo.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
struct Tip {
    #[n(0)]
    wallet_utxos: BTreeMap<Input, Output>,
    #[n(1)]
    channels: BTreeMap<Input, (Output, Channel)>,
}

impl Cache {
    pub fn network_parameters(&self) -> Option<NetworkParameters> {
        self.network_parameters.clone()
    }
    pub fn set_network_parameters(&mut self, network_parameters: NetworkParameters) {
        self.network_parameters = Some(network_parameters);
    }
    pub fn reference_script(&self) -> Option<Utxo> {
        self.reference_script.clone()
    }
    pub fn set_reference_script(&mut self, reference_script: Utxo) {
        self.reference_script = Some(reference_script);
    }

    pub fn channels(&self) -> BTreeMap<Input, (Output, Channel)> {
        self.tip.channels.clone()
    }
    pub fn wallet_utxos(&self) -> BTreeMap<Input, Output> {
        self.tip.wallet_utxos.clone()
    }

    /// Replace the cached tip. Returns the set of inputs that no longer
    /// have a live channel, so the caller (`L1::set_tip`) can drop any
    /// directives referring to them — keeping that reconciliation at the
    /// `L1` level, since `Cache` alone doesn't know about `Directives`.
    pub fn set_tip(
        &mut self,
        wallet_utxos: BTreeMap<Input, Output>,
        channels: BTreeMap<Input, (Output, Channel)>,
    ) -> BTreeSet<Input> {
        let live_inputs: BTreeSet<Input> = channels.keys().cloned().collect();
        self.tip.wallet_utxos = wallet_utxos;
        self.tip.channels = channels;
        live_inputs
    }

    pub fn built_tx(&self) -> Option<Transaction<ReadyForSigning>> {
        self.built_tx.clone()
    }
    pub fn set_built_tx(&mut self, tx: Transaction<ReadyForSigning>) {
        self.built_tx = Some(tx);
    }
    pub fn clear_built_tx(&mut self) {
        self.built_tx = None;
    }
}
