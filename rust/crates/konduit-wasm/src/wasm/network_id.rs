use crate::{core, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone, Copy)]
    #[doc = "A network identifier to protect misuses of addresses or transactions on a wrong network."]
    NetworkId => core::NetworkId
}

#[wasm_bindgen]
impl NetworkId {
    #[wasm_bindgen(js_name = "mainnet")]
    pub fn _wasm_mainnet() -> Self {
        Self(core::NetworkId::MAINNET)
    }

    #[wasm_bindgen(js_name = "testnet")]
    pub fn _wasm_testnet() -> Self {
        Self(core::NetworkId::TESTNET)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}
