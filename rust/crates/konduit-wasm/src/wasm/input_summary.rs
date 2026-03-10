use crate::{
    core,
    wasm::{Input, OutputSummary},
    wasm_proxy,
};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "An `Input` alongside its resolved `Output`"]
    InputSummary => core::InputSummary
}

#[wasm_bindgen]
impl InputSummary {
    #[wasm_bindgen(getter, js_name = "input")]
    pub fn input(&self) -> Input {
        self.input.clone().into()
    }

    #[wasm_bindgen(getter, js_name = "output")]
    pub fn output(&self) -> OutputSummary {
        self.output.clone().into()
    }
}
