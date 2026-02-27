use crate::types::OutputSummary;
use cardano_sdk::Input;

#[derive(Debug, Clone)]
pub struct InputSummary {
    pub input: Input,
    pub output: OutputSummary,
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::types::wasm::OutputSummary;
    use cardano_sdk::{wasm::Input, wasm_proxy};
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[derive(Debug, Clone)]
        #[doc = "An `Input` alongside its resolved `Output`"]
        InputSummary
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
}
