use crate::{core, wasm::Tag, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A channel output as visible on the Cardano L1"]
    ChannelOutput => core::Channel
}

#[wasm_bindgen]
impl ChannelOutput {
    /// Indicate whether the channel is 'Opened'
    #[wasm_bindgen(getter, js_name = "isOpened")]
    pub fn _wasm_is_opened(&self) -> bool {
        matches!(self.stage(), core::Stage::Opened { .. })
    }

    /// Indicate whether the channel is 'Closed'
    #[wasm_bindgen(getter, js_name = "isClosed")]
    pub fn _wasm_is_closed(&self) -> bool {
        matches!(self.stage(), core::Stage::Closed { .. })
    }

    /// Indicate whether the channel is 'Responded'
    #[wasm_bindgen(getter, js_name = "isResponded")]
    pub fn _wasm_is_responded(&self) -> bool {
        matches!(self.stage(), core::Stage::Responded { .. })
    }

    /// Return the channel tag.
    #[wasm_bindgen(getter, js_name = "tag")]
    pub fn _wasm_tag(&self) -> Tag {
        self.constants().tag.clone().into()
    }

    /// Return the total amount already subbed from the channel
    #[wasm_bindgen(getter, js_name = "subbedAmount")]
    pub fn _wasm_subbed_amount(&self) -> u64 {
        match self.stage() {
            core::Stage::Opened(subbed, _)
            | core::Stage::Closed(subbed, _, _)
            | core::Stage::Responded(subbed, _) => *subbed,
        }
    }

    /// Return the total amount deposited in the channel. Owed amount is obtained by looking at the
    /// receipt.
    #[wasm_bindgen(getter, js_name = "totalAmount")]
    pub fn _wasm_total_amount(&self) -> u64 {
        self.amount() + self._wasm_subbed_amount() + core::MIN_ADA_BUFFER
    }
}
