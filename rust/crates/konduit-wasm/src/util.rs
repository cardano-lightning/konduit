use cardano_tx_builder::{SigningKey, VerificationKey};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "toVerificationKey")]
pub fn to_verification_key(signing_key: &[u8]) -> Vec<u8> {
    let signing_key: SigningKey = <[u8; 32]>::try_from(signing_key)
        .expect("invalid verification key length")
        .into();

    Vec::from(VerificationKey::from(&signing_key).as_ref())
}
