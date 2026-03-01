use crate::{core, wasm::VerificationKey, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "Channel parameters and ToS of a given adaptor."]
    AdaptorInfo => core::AdaptorInfo
}

#[wasm_bindgen]
impl AdaptorInfo {
    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn _wasm_verification_key(&self) -> VerificationKey {
        self.channel_parameters.adaptor_key.into()
    }

    #[wasm_bindgen(getter, js_name = "closePeriod")]
    pub fn _wasm_close_period(&self) -> u64 {
        self.channel_parameters.close_period.as_secs()
    }

    #[wasm_bindgen(getter, js_name = "maxTagLength")]
    pub fn _wasm_max_tag_length(&self) -> u8 {
        self.channel_parameters.tag_length as u8
    }

    #[wasm_bindgen(getter, js_name = "fee")]
    pub fn _wasm_fee(&self) -> u64 {
        self.tos.flat_fee
    }
}
