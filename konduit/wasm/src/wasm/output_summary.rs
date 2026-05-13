use crate::{
    core,
    wasm::{Hash28, ShelleyAddress},
    wasm_proxy,
};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "An `Output` that carries only reference scripts as hashes, and not plain scripts."]
    OutputSummary => core::OutputSummary
}

#[wasm_bindgen]
impl OutputSummary {
    #[wasm_bindgen(getter, js_name = "address")]
    pub fn _wasm_address(&self) -> ShelleyAddress {
        self.address()
            .as_shelley()
            .expect("Byron address found in OutputSummary")
            .into()
    }

    #[wasm_bindgen(getter, js_name = "lovelace")]
    pub fn _wasm_lovelace(&self) -> u64 {
        self.value().lovelace()
    }

    #[wasm_bindgen(getter, js_name = "script")]
    pub fn _wasm_script(&self) -> Option<Hash28> {
        self.script().map(Into::into)
    }
}
