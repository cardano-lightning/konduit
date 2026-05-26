use crate::{core, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone, Copy)]
    #[doc = "A blake-2b hash digest of 28 bytes (224 bits)"]
    Hash28 => core::Hash<28>
}

#[wasm_bindgen]
impl Hash28 {
    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}
