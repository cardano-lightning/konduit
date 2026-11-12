use crate::{
    core,
    core::{AdaptorChannelParameters, Duration},
    wasm::VerificationKey,
    wasm_proxy,
};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "Channel parameters and ToS of a given adaptor."]
    AdaptorInfo => core::info::Response<()>
}

#[wasm_bindgen]
impl AdaptorInfo {
    #[wasm_bindgen(constructor)]
    pub fn _wasm_new(
        adaptor_key: VerificationKey,
        close_period: u64,
        max_tag_length: u8,
        fee: u64,
        time_margin: u64,
    ) -> Self {
        Self(core::info::Response {
            channel_parameters: AdaptorChannelParameters {
                adaptor_key: core::to_verifying_key(adaptor_key.into()),
                close_period: Duration::from_secs(close_period),
                tag_length: max_tag_length as usize,
            },
            tos: core::info::TosInfo {
                flat_fee: fee,
                time_margin: Duration::from_secs(time_margin),
                rails: Vec::new(),
            },
            tx_help: (),
        })
    }

    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn _wasm_verification_key(&self) -> VerificationKey {
        core::from_verifying_key(self.channel_parameters.adaptor_key).into()
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

    /// Konduit's own time margin: how much sooner than the cheque the
    /// outbound commitment must resolve.
    #[wasm_bindgen(getter, js_name = "timeMargin")]
    pub fn _wasm_time_margin(&self) -> u64 {
        self.tos.time_margin.as_secs()
    }

    /// The rails this adaptor is configured with, for the consumer to
    /// pick one per payment.
    #[wasm_bindgen(getter, js_name = "rails")]
    pub fn _wasm_rails(&self) -> Vec<String> {
        self.tos.rails.clone()
    }
}
