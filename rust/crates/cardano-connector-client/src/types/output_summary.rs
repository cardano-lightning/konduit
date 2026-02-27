use cardano_sdk::{Address, Datum, Hash, Output, Value, address::kind};

/// An `Output` that carries only reference scripts as hashes, and not plain scripts.
#[derive(Debug, Clone)]
// TODO: possibly unify 'OutputSummary' and 'Output'
// Instead of having yet-another-indirection. The main problem here is how we want to have outputs
// that do not carry around full scripts. Maybe that's just something we can accept, but that means
// more queries from APIs too..
pub struct OutputSummary {
    output: Output,
    reference_script_hash: Option<Hash<28>>,
}

impl OutputSummary {
    pub fn new(output: Output, reference_script_hash: Option<Hash<28>>) -> OutputSummary {
        Self {
            output,
            reference_script_hash,
        }
    }

    pub fn address(&self) -> &Address<kind::Any> {
        self.output.address()
    }

    pub fn value(&self) -> &Value<u64> {
        self.output.value()
    }

    pub fn datum(&self) -> Option<&Datum> {
        self.output.datum()
    }

    pub fn script(&self) -> Option<Hash<28>> {
        self.reference_script_hash
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use cardano_sdk::{
        wasm::{Hash28, ShelleyAddress},
        wasm_proxy,
    };
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[derive(Debug, Clone)]
        #[doc = "An `Output` that carries only reference scripts as hashes, and not plain scripts."]
        OutputSummary
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
            self.reference_script_hash.map(|h| h.into())
        }
    }
}
