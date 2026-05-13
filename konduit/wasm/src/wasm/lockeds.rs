use crate::{
    core,
    wasm::{self, Lock},
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[doc = "A list of known locks the consumer expect to see while syncing with the adaptor."]
pub struct Lockeds(Option<BTreeSet<core::Lock>>);

impl Lockeds {
    pub fn add(&mut self, lock: core::Lock) {
        if let Some(locks) = self.0.as_mut() {
            locks.insert(lock);
        } else {
            self.0 = Some(BTreeSet::from([lock]));
        }
    }

    pub fn reset(&mut self, locks: BTreeSet<core::Lock>) {
        self.0 = Some(locks);
    }

    /// A predicate to filter out unlocked from receipt or squash proposal that are unexpected. A
    /// cheque is unexpected if it "reappear" from a past conversation with the adaptor. That is,
    /// if an adaptor has forfeited a locked cheque because it has timed out, it cannot attempt to
    /// later try to include it in a squash.
    ///
    /// Note that this still allows an adaptor to hold onto an expired cheque, should they want to
    /// wait for a little longer than the calculated timeout. This can be useful if, for example,
    /// if an adaptor has subbed but is now trying to squash the cheque so that the on-chain state
    /// can be reset to a clean state (since the number of rogue cheques like this is limited by
    /// the smart contracts).
    ///
    /// So in practice, a consumer tracks the last "locked" cheques it receives from an adaptor,
    /// and expects unlocked cheques to necessarily be in this list. As soon as an adaptor drops a
    /// cheque from the locked cheques, the consumer can reasonably assume it is expired and that
    /// it won't ever appear. If it does, then clearly the adaptor is doing something fishy.
    pub fn as_filter(&self) -> impl Fn(core::Lock) -> bool {
        |lock| match self.0.as_ref() {
            Some(locks) => locks.contains(&lock),
            None => true,
        }
    }
}

#[wasm_bindgen]
impl Lockeds {
    #[wasm_bindgen(constructor)]
    pub fn _wasm_new() -> Self {
        Self(None)
    }

    #[wasm_bindgen(js_name = "has")]
    pub fn _wasm_has(&self, lock: &Lock) -> bool {
        match self.0.as_ref() {
            None => false,
            Some(locks) => locks.contains(lock),
        }
    }

    #[wasm_bindgen(js_name = "serialize")]
    pub fn _wasm_serialize(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|e| unreachable!("failed to serialized Locked to JSON: {e}"))
    }

    #[wasm_bindgen(js_name = "deserialize")]
    pub fn _wasm_deserialize(value: &str) -> wasm::Result<Self> {
        serde_json::from_str(value)
            .map_err(|e| anyhow!("failed to deserialize Lockeds from JSON: {e}").into())
    }
}
