use crate::{Client, Transport, codec, transport, url};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GlooClient {
    inner: Client<transport::Gloo, codec::Json>,
}

#[wasm_bindgen]
impl GlooClient {
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, timeout_ms: Option<u32>) -> Self {
        let timeout = timeout_ms.map(|ms| web_time::Duration::from_millis(ms as u64));
        Self {
            inner: Client::new(transport::Gloo::new(timeout), codec::Json, base_url),
        }
    }

    #[wasm_bindgen]
    pub async fn get(&self, path: String) -> Result<js_sys::Uint8Array, JsValue> {
        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri(url::clean_join(&self.inner.base_url, &path))
            .body(Vec::new())
            .map_err(js_err)?;

        let res = self.inner.transport.transport(req).await.map_err(js_err)?;

        Ok(js_sys::Uint8Array::from(res.into_body().as_slice()))
    }
}

fn js_err(e: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
