use crate::{core, wasm, wasm_proxy};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone, Copy)]
    #[doc = "An Ed25519 verification key (non-extended)."]
    VerificationKey => core::VerificationKey
}

#[wasm_bindgen]
impl VerificationKey {
    #[wasm_bindgen(js_name = "tryParse")]
    /// Construct a `VerificationKey` from a 64-digit hex-encoded text string. Throws if the
    /// string is malformed.
    pub fn _wasm_try_parse(value: &str) -> wasm::Result<Self> {
        Ok(core::VerificationKey::from_str(value)?.into())
    }

    #[wasm_bindgen(js_name = "toString")]
    /// Encode the `VerificationKey` as a 64-digit hex-encoded text string.
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}
