use crate::{
    core,
    wasm::{Hash32, InputSummary, OutputSummary},
    wasm_proxy,
};
use wasm_bindgen::{JsValue, prelude::*};

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A synthetic representation of a transaction used by the Connector."]
    TransactionSummary => core::TransactionSummary
}

#[wasm_bindgen]
impl TransactionSummary {
    #[wasm_bindgen(getter, js_name = "id")]
    pub fn _wasm_id(&self) -> Hash32 {
        Hash32::from(self.id)
    }

    #[wasm_bindgen(getter, js_name = "index")]
    pub fn _wasm_index(&self) -> u64 {
        self.index
    }

    #[wasm_bindgen(getter, js_name = "depth")]
    pub fn _wasm_depth(&self) -> u64 {
        self.depth
    }

    #[wasm_bindgen(getter, js_name = "outputs")]
    pub fn _wasm_outputs(&self) -> Vec<OutputSummary> {
        self.outputs
            .iter()
            .map(|(partial_output, reference_script_hash)| {
                core::OutputSummary::new(partial_output.clone(), *reference_script_hash).into()
            })
            .collect()
    }

    #[wasm_bindgen(getter, js_name = "inputs")]
    pub fn _wasm_inputs(&self) -> Vec<InputSummary> {
        self.inputs
            .iter()
            .map(|(input, partial_output, reference_script_hash)| {
                core::InputSummary {
                    input: input.clone(),
                    output: core::OutputSummary::new(
                        partial_output.clone(),
                        *reference_script_hash,
                    ),
                }
                .into()
            })
            .collect()
    }

    #[wasm_bindgen(getter, js_name = "timestamp")]
    pub fn _wasm_timestamp(&self) -> js_sys::Date {
        js_sys::Date::new(&JsValue::from_f64((self.timestamp_secs * 1000) as f64))
    }
}
