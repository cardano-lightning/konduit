use crate::{core, wasm, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A CLN-network payment request returned by a merchant."]
    Payme => core::Payme
}

#[wasm_bindgen]
impl Payme {
    #[wasm_bindgen(js_name = "fromJson")]
    pub fn _wasm_from_json(json: &str) -> wasm::Result<Self> {
        let payme: core::Payme = serde_json::from_str(json).map_err(anyhow::Error::from)?;
        Ok(payme.into())
    }

    #[wasm_bindgen(getter, js_name = "payee")]
    pub fn _wasm_payee(&self) -> String {
        hex::encode(<[u8; 32]>::from(self.payee))
    }

    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn _wasm_amount(&self) -> u64 {
        self.amount
    }

    #[wasm_bindgen(getter, js_name = "lock")]
    pub fn _wasm_lock(&self) -> String {
        hex::encode(self.lock.0)
    }

    #[wasm_bindgen(getter, js_name = "timeout")]
    pub fn _wasm_timeout(&self) -> u64 {
        self.timeout.as_millis() as u64
    }
}
