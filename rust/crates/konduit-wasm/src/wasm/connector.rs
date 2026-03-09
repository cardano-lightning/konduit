use crate::{HttpClient, wasm, wasm_proxy};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Clone)]
    #[doc = "A reference to a Cardano connector."]
    Connector => Rc<crate::Connector<HttpClient>>
}

#[wasm_bindgen]
impl Connector {
    #[wasm_bindgen(js_name = "new")]
    pub async fn _wasm_new(url: &str) -> wasm::Result<Self> {
        Ok(Self(Rc::new(
            crate::Connector::new(HttpClient::new(url)).await?,
        )))
    }
}
