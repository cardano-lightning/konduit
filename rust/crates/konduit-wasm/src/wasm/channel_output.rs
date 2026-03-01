use crate::{core, wasm::Tag, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A channel output as visible on the Cardano L1"]
    ChannelOutput => core::ChannelOutput
}

#[wasm_bindgen]
impl ChannelOutput {
    /// Return the channel tag.
    #[wasm_bindgen(getter, js_name = "tag")]
    pub fn _wasm_tag(&self) -> Tag {
        self.constants.tag.clone().into()
    }

    /// Return the initial amount deposited in the channel. Owed amount is obtained elsewhere.
    #[wasm_bindgen(getter, js_name = "initialAmount")]
    pub fn _wasm_initial_amount(&self) -> u64 {
        let subbed = match self.0.stage {
            core::Stage::Opened(subbed, _)
            | core::Stage::Closed(subbed, _, _)
            | core::Stage::Responded(subbed, _) => subbed,
        };

        self.0.amount + subbed + core::MIN_ADA_BUFFER
    }
}
