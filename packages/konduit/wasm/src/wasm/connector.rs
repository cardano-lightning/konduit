use crate::{new_http_client, wasm, wasm_proxy};
use http_client::GlooTransport;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Clone)]
    #[doc = "A reference to a Cardano connector."]
    Connector => Rc<crate::Connector<GlooTransport>>
}

#[wasm_bindgen]
impl Connector {
    #[wasm_bindgen(js_name = "new")]
    pub async fn _wasm_new(url: &str) -> wasm::Result<Self> {
        Ok(Self(Rc::new(
            crate::Connector::new(new_http_client(url)).await?,
        )))
    }
}
