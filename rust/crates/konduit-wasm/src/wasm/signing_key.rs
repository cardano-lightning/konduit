use crate::{core, wasm, wasm_proxy};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "An Ed25519 signing key (non-extended)."]
    SigningKey => core::SigningKey
}

#[wasm_bindgen]
impl SigningKey {
    #[wasm_bindgen(constructor)]
    pub fn _wasm_new() -> Self {
        core::SigningKey::new().into()
    }

    #[wasm_bindgen(js_name = "tryParse")]
    pub fn _wasm_parse(value: &str) -> wasm::Result<Self> {
        Ok(core::SigningKey::from_str(value)?.into())
    }
}
