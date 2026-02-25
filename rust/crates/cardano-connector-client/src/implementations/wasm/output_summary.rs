use cardano_sdk::{address::ShelleyAddress, hash::Hash28};
use wasm_bindgen::prelude::*;

// FIXME: Offer a proper API for Output in wasm
//
// Instead of having yet-another-indirection. The main problem here is how we want to have outputs
// that do not carry around full scripts. Maybe that's just something we can accept, but that means
// more queries from APIs too..
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct OutputSummary {
    pub(crate) address: ShelleyAddress,
    pub lovelace: u64,
    pub reference_script_hash: Option<Hash28>,
}

#[wasm_bindgen]
impl OutputSummary {
    #[wasm_bindgen(getter, js_name = "address")]
    pub fn address(&self) -> ShelleyAddress {
        self.address.clone()
    }
}
