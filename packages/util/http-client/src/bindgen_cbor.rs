use crate::CborCodec;
use crate::bindgen::{js_err, make_get_request};
use crate::prelude::*;
use crate::{Client, GlooTransport, Transport};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CborClient {
    inner: Client<GlooTransport, CborCodec>,
}

#[wasm_bindgen]
impl CborClient {
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, timeout_ms: Option<u32>) -> Self {
        let timeout = timeout_ms.map(|ms| web_time::Duration::from_millis(ms as u64));
        Self {
            inner: Client::new(GlooTransport::new(timeout), CborCodec, base_url),
        }
    }

    #[wasm_bindgen]
    pub async fn get(&self, path: String) -> Result<js_sys::Uint8Array, JsValue> {
        let req = make_get_request(&self.inner.base_url, &path)?;
        let resp = self.inner.transport.transport(req).await.map_err(js_err)?;
        Ok(js_sys::Uint8Array::from(resp.into_body().as_slice()))
    }
}
