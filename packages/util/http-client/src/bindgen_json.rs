use crate::JsonCodec;
use crate::bindgen::{js_err, make_get_request};
use crate::prelude::*;
use crate::{Client, GlooTransport, Transport};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct JsonClient {
    inner: Client<GlooTransport, JsonCodec>,
}

#[wasm_bindgen]
impl JsonClient {
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, timeout_ms: Option<u32>) -> Self {
        let timeout = timeout_ms.map(|ms| web_time::Duration::from_millis(ms as u64));
        Self {
            inner: Client::new(GlooTransport::new(timeout), JsonCodec, base_url),
        }
    }

    #[wasm_bindgen]
    pub async fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let req = make_get_request(&self.inner.base_url, &path)?;
        let resp = self.inner.transport.transport(req).await.map_err(js_err)?;
        let body = resp.into_body();
        let json_str = core::str::from_utf8(&body).map_err(js_err)?;
        Ok(JsValue::from_str(json_str))
    }
}
