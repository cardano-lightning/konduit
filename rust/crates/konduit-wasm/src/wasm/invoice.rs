use crate::{core, wasm, wasm::Lock, wasm_proxy};
use anyhow::Context;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A parsed Bolt11 invoice"]
    Invoice => core::Invoice
}

#[wasm_bindgen]
impl Invoice {
    #[wasm_bindgen(js_name = "tryParse")]
    pub fn _wasm_try_parse(value: &str) -> wasm::Result<Self> {
        Ok(Self(
            value.parse().context("failed to parse bolt11 invoice")?,
        ))
    }

    #[wasm_bindgen(getter, js_name = "lock")]
    pub fn _wasm_lock(&self) -> Lock {
        core::Lock(self.payment_hash).into()
    }
}
