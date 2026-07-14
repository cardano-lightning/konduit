use crate::{new_http_client, wasm, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[doc = "A Konduit Adaptor."]
    Adaptor => crate::Adaptor
}

#[wasm_bindgen]
impl Adaptor {
    #[wasm_bindgen(js_name = "new")]
    pub async fn _wasm_new(url: &str) -> wasm::Result<Self> {
        Ok(Self(crate::Adaptor::new(new_http_client(url), None).await?))
    }
}
