use konduit_data::{Constants, Duration, Tag, VerifyingKey};
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::core::Input;

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OpenIntent {
    #[n(0)]
    pub tag: Tag,
    #[n(1)]
    pub sub_vkey: VerifyingKey,
    #[n(2)]
    pub close_period: Duration,
    #[n(3)]
    pub amount: u64,
}

impl OpenIntent {
    pub(crate) fn constants(self, add_vkey: VerifyingKey) -> Constants {
        Constants {
            tag: self.tag,
            add_vkey,
            sub_vkey: self.sub_vkey,
            close_period: self.close_period,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Intent {
    #[n(0)]
    Add(#[n(0)] u64),
    #[n(1)]
    Close,
}

/// Pending build directives — user-stated intent with no chain-side
/// source to recover it from if lost: opens keyed by tag, per-input
/// intent, and — when `Config::autocomplete` is `false` — the set of
/// inputs to force-expire, -elapse, or -end. Ignored entirely when
/// `autocomplete` is `true`.
#[derive(Debug, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Directives {
    #[n(0)]
    opens: BTreeMap<Tag, OpenIntent>,
    #[n(1)]
    intents: BTreeMap<Input, Intent>,
    #[n(2)]
    force: BTreeSet<Input>,
}

impl Directives {
    pub fn intent(&self, input: &Input) -> Option<Intent> {
        self.intents.get(input).cloned()
    }
    pub fn intents(&self) -> BTreeMap<Input, Intent> {
        self.intents.clone()
    }
    pub fn add_intent(&mut self, input: Input, intent: Intent) {
        self.intents.insert(input, intent);
    }
    pub fn remove_intent(&mut self, input: &Input) {
        self.intents.remove(input);
    }
    pub fn clear_intents(&mut self) {
        self.intents.clear();
    }

    pub fn force(&self) -> BTreeSet<Input> {
        self.force.clone()
    }
    pub fn add_force(&mut self, input: Input) {
        self.force.insert(input);
    }
    pub fn remove_force(&mut self, input: &Input) {
        self.force.remove(input);
    }
    pub fn clear_force(&mut self) {
        self.force.clear();
    }

    pub fn opens(&self) -> BTreeMap<Tag, OpenIntent> {
        self.opens.clone()
    }
    pub fn add_open(&mut self, open: OpenIntent) {
        self.opens.insert(open.tag.clone(), open);
    }
    pub fn remove_open(&mut self, tag: &Tag) {
        self.opens.remove(tag);
    }
    pub fn clear_opens(&mut self) {
        self.opens.clear();
    }

    /// Drop any intent/force entry for an input no longer in `live_inputs`
    /// — called from `L1::set_tip` after the chain-pulled tip changes, so
    /// cached intent never drifts out of sync with the channels it refers
    /// to.
    pub(crate) fn retain_live(&mut self, live_inputs: &BTreeSet<Input>) {
        self.intents.retain(|input, _| live_inputs.contains(input));
        self.force.retain(|input| live_inputs.contains(input));
    }
}
