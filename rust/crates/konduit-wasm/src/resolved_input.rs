use cardano_tx_builder::{Input, Output};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug)]
pub struct ResolvedInput {
    input: Input,
    output: Output,
}

impl ResolvedInput {
    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn output(&self) -> &Output {
        &self.output
    }
}

#[wasm_bindgen]
impl ResolvedInput {
    #[wasm_bindgen(constructor)]
    pub fn new(input: &Input, output: &Output) -> Self {
        Self {
            input: input.clone(),
            output: output.clone(),
        }
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        format!("{self:#?}")
    }
}

#[wasm_bindgen]
#[derive(Debug)]
pub struct ResolvedInputs(Vec<ResolvedInput>);

impl ResolvedInputs {
    pub fn iter(&self) -> impl Iterator<Item = (&Input, &Output)> {
        self.0.iter().map(|r| (&r.input, &r.output))
    }
}

#[wasm_bindgen]
impl ResolvedInputs {
    #[wasm_bindgen]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    #[wasm_bindgen]
    pub fn append(mut self, resolved_input: ResolvedInput) -> Self {
        self.0.push(resolved_input);
        self
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        format!("{self:#?}")
    }
}
