use crate::{
    core,
    wasm::{self, Credential},
    wasm_proxy,
};
use std::{ops::Deref, str::FromStr};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "An `Address`, specialized to the `Shelley` kind."]
    ShelleyAddress => core::Address<core::address::kind::Shelley>
}

#[wasm_bindgen]
impl ShelleyAddress {
    #[wasm_bindgen(js_name = "equals")]
    pub fn _wasm_equals(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }

    #[wasm_bindgen(js_name = "tryParse")]
    pub fn _wasm_try_parse(addr: &str) -> wasm::Result<Self> {
        Ok(core::Address::from_str(addr)?.into())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        self.0.to_string()
    }

    #[wasm_bindgen(getter, js_name = "paymentCredential")]
    pub fn _wasm_payment_credential(&self) -> Credential {
        self.payment().into()
    }

    #[wasm_bindgen(getter, js_name = "delegationCredential")]
    pub fn _wasm_delegation_credential(&self) -> Option<Credential> {
        self.delegation().map(Into::into)
    }
}
