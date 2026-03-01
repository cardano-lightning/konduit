use crate::{
    core,
    wasm::{self, Hash28, NetworkId},
    wasm_proxy,
};
use std::{ops::Deref, str::FromStr};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A wrapper around the blake2b-224 hash digest of a key or script."]
    Credential => core::Credential
}

#[wasm_bindgen]
impl Credential {
    #[wasm_bindgen(constructor)]
    pub fn _wasm_new(credential: &str) -> wasm::Result<Self> {
        Ok(Self(core::Credential::from_str(credential)?))
    }

    #[wasm_bindgen(js_name = "equals")]
    pub fn _wasm_equals(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }

    #[wasm_bindgen(js_name = "toStringWithNetworkId")]
    pub fn _wasm_to_string_with_network_id(&self, network_id: NetworkId) -> String {
        core::WithNetworkId {
            inner: self.deref(),
            network_id: network_id.into(),
        }
        .to_string()
    }

    #[wasm_bindgen(js_name = "asKey")]
    pub fn _wasm_as_key(&self) -> Option<Hash28> {
        self.as_key().map(Into::into)
    }

    #[wasm_bindgen(js_name = "asScript")]
    pub fn _wasm_as_script(&self) -> Option<Hash28> {
        self.as_script().map(Into::into)
    }
}
