use crate::{core, wasm::Hash32, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A reference to a past transaction output."]
    Input => core::Input
}

#[wasm_bindgen]
impl Input {
    #[wasm_bindgen(constructor)]
    pub fn _wasm_new(transaction_id: &Hash32, output_index: u64) -> Self {
        core::Input::new((*transaction_id).into(), output_index).into()
    }

    #[wasm_bindgen(getter, js_name = "transactionId")]
    pub fn _wasm_transaction_id(&self) -> Hash32 {
        self.transaction_id().into()
    }

    #[wasm_bindgen(getter, js_name = "outputIndex")]
    pub fn _wasm_output_index(&self) -> u64 {
        self.output_index()
    }
}
