use crate::{core, wasm, wasm_proxy};
use anyhow::anyhow;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "An up-to-32-bytes tag to allow reuse of `VerificationKey` across channels."]
    Tag => core::Tag
}

#[wasm_bindgen]
impl Tag {
    #[wasm_bindgen(js_name = "tryParse")]
    pub fn _wasm_try_parse(value: &str) -> wasm::Result<Self> {
        Ok(Self(core::Tag::from_str(value).map_err(|e| anyhow!(e))?))
    }

    #[wasm_bindgen(js_name = "generate")]
    pub fn _wasm_generate(length: usize) -> Self {
        Self(core::Tag::generate(length))
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}
