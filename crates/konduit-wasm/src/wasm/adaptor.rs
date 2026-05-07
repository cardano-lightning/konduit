use crate::{HttpClient, wasm, wasm_proxy};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[doc = "A Konduit Adaptor."]
    Adaptor => crate::Adaptor<HttpClient>
}

#[wasm_bindgen]
impl Adaptor {
    #[wasm_bindgen(js_name = "new")]
    pub async fn _wasm_new(url: &str) -> wasm::Result<Self> {
        Ok(Self(crate::Adaptor::new(HttpClient::new(url), None).await?))
    }
}
