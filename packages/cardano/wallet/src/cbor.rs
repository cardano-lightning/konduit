#![cfg(any(target_arch = "wasm32", feature = "cli"))]
//! Hex <-> CBOR via minicbor, shared by `cip30` and `web`. Error is a
//! plain `String` - callers map it into their own error type.

pub(crate) fn from_cbor_hex<T: for<'a> minicbor::Decode<'a, ()>>(hex: &str) -> Result<T, String> {
    minicbor::decode(&hex::decode(hex).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

// FIXME :: Not used??
// pub(crate) fn to_cbor_hex<T: minicbor::Encode<()>>(value: &T) -> Result<String, String> {
//     minicbor::to_vec(value).map(hex::encode).map_err(|e| e.to_string())
// }
