use crate::{
    core, wasm,
    wasm::{Hash32, InputSummary, OutputSummary},
    wasm_proxy,
};
use anyhow::anyhow;
use core::cbor::{FromCbor, ToCbor};
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

    #[wasm_bindgen(js_name = "serialise")]
    pub fn _wasm_serialise(&self) -> String {
        hex::encode(self.to_cbor())
    }

    #[wasm_bindgen(js_name = "deserialise")]
    pub fn _wasm_deserialise(s: &str) -> wasm::Result<Self> {
        let bytes = hex::decode(s).map_err(|e| anyhow!("invalid hex: {e}"))?;
        Ok(Self(
            core::TransactionSummary::from_cbor(&bytes)
                .map_err(|e| anyhow!("invalid bytes: {e}"))?,
        ))
    }
}
