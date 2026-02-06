use anyhow::anyhow;
use cardano_connect_wasm::{self as wasm};
use cardano_tx_builder::{SigningKey, VerificationKey};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "toVerificationKey")]
pub fn to_verification_key(signing_key: &[u8]) -> wasm::Result<Vec<u8>> {
    let signing_key: SigningKey = <[u8; 32]>::try_from(signing_key)
        .map_err(|_| anyhow!("Invalid signing key length, expected 32 bytes"))?
        .into();
    Ok(Vec::from(VerificationKey::from(&signing_key).as_ref()))
}
