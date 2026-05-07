use crate::{core, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone, Copy)]
    #[doc = "A blake-2b hash digest of 32 bytes (256 bits)"]
    Hash32 => core::Hash<32>
}

#[wasm_bindgen]
impl Hash32 {
    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}
