use crate::{
    core,
    wasm::{self},
    wasm_proxy,
};
use std::ops::Deref;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone, Copy)]
    #[doc = "The hash of an HTLC secret, awaiting a preimage."]
    Lock => core::Lock
}

#[wasm_bindgen]
impl Lock {
    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }

    #[wasm_bindgen(js_name = "tryParse")]
    pub fn _wasm_try_parse(value: &str) -> wasm::Result<Self> {
        Ok(Self(
            core::Lock::try_from(hex::decode(value).map_err(|_| anyhow::anyhow!("hex error"))?)
                .map_err(|_| anyhow::anyhow!("lock from vec error"))?,
        ))
    }

    #[wasm_bindgen(js_name = "equals")]
    pub fn _wasm_equals(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
