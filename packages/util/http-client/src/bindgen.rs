use crate::prelude::*;
use crate::url;
use wasm_bindgen::prelude::*;

pub(crate) fn js_err(e: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

pub(crate) fn make_get_request(
    base_url: &str,
    path: &str,
) -> Result<http::Request<Vec<u8>>, JsValue> {
    http::Request::builder()
        .method(http::Method::GET)
        .uri(url::clean_join(base_url, path))
        .body(Vec::new())
        .map_err(js_err)
}
