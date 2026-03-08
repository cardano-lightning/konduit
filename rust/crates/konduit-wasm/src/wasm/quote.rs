use crate::{core, wasm::Lock, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "Proposed price and routing fee by an Adaptor for a given Bolt11 invoice."]
    Quote => core::Quote
}

#[wasm_bindgen]
impl Quote {
    #[wasm_bindgen(getter, js_name = "index")]
    pub fn _wasm_index(&self) -> u64 {
        self.index
    }

    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn _wasm_amount(&self) -> u64 {
        self.amount
    }

    #[wasm_bindgen(getter, js_name = "relativeTimeout")]
    pub fn _wasm_relative_timeout(&self) -> u64 {
        self.relative_timeout
    }

    #[wasm_bindgen(getter, js_name = "routingFee")]
    pub fn _wasm_routing_fee(&self) -> u64 {
        self.routing_fee
    }

    #[wasm_bindgen(getter, js_name = "lock")]
    pub fn _wasm_lock(&self) -> Lock {
        self.lock.into()
    }
}
