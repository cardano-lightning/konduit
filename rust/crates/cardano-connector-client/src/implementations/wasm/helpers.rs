use anyhow::anyhow;
use wasm_bindgen::prelude::*;

pub fn try_into_array<T, const N: usize>(v: Vec<T>) -> anyhow::Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}

pub fn singleton(key: &str, value: impl Into<JsValue>) -> anyhow::Result<JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &(key.into()), &(value.into())).map_err(|e| {
        anyhow!(
            "failed to construct singleton object with key '{}': {:?}",
            key,
            e
        )
    })?;
    Ok(obj.into())
}

pub fn to_js_object(kvs: &[(&str, JsValue)]) -> anyhow::Result<JsValue> {
    let obj = js_sys::Object::new();
    for (key, value) in kvs {
        js_sys::Reflect::set(&obj, &((*key).into()), value).map_err(|e| {
            anyhow!(
                "failed to construct singleton object with key '{}': {:?}",
                key,
                e
            )
        })?;
    }
    Ok(obj.into())
}
