use crate::JsonCodec;
use crate::prelude::*;
use crate::{GlooTransport, HttpClient, HttpTransport, url};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GlooClient {
    inner: HttpClient<GlooTransport, JsonCodec>,
}

#[wasm_bindgen]
impl GlooClient {
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, timeout_ms: Option<u32>) -> Self {
        let timeout = timeout_ms.map(|ms| web_time::Duration::from_millis(ms as u64));
        Self {
            inner: HttpClient::new(GlooTransport::new(timeout), JsonCodec, base_url),
        }
    }

    #[wasm_bindgen]
    pub async fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri(url::clean_join(&self.inner.base_url, &path))
            .body(Vec::new())
            .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;

        let resp = self
            .inner
            .transport
            .transport(req)
            .await
            .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;

        let body = resp.into_body();

        let json_str =
            core::str::from_utf8(&body).map_err(|e| JsValue::from_str(e.to_string().as_str()))?;

        Ok(JsValue::from_str(json_str))
    }
}
