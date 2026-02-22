use crate::OutputSummary;
use cardano_tx_builder::Input;
use wasm_bindgen::prelude::*;

// FIXME: Offer a proper API for Input in wasm
//
// Instead of having yet-another-indirection. The main problem here is how we want to have outputs
// that do not carry around full scripts. Maybe that's just something we can accept, but that means
// more queries from APIs too..
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct InputSummary {
    pub(crate) input: Input,
    pub(crate) output: OutputSummary,
}

#[wasm_bindgen]
impl InputSummary {
    #[wasm_bindgen(getter, js_name = "input")]
    pub fn input(&self) -> Input {
        self.input.clone()
    }

    #[wasm_bindgen(getter, js_name = "output")]
    pub fn output(&self) -> OutputSummary {
        self.output.clone()
    }
}
