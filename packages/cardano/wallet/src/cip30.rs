#![cfg(target_arch = "wasm32")]
//! CIP-30 wallet backend: extension owns keys, UTXO selection, submission.
//! Everything crosses the JS boundary as hex CBOR.

use crate::{cbor, wallet::Wallet};
use cardano_sdk::{
    Address, Hash, Input, NetworkId, Output, Signature, Transaction, Value, VerificationKey,
    address::kind, transaction::state,
};
use std::collections::BTreeMap;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no CIP-30 wallet named {0:?} found on `window.cardano`")]
    NotFound(String),
    #[error("wallet extension error: {0}")]
    Js(String),
    #[error("malformed response from wallet extension: {0}")]
    Decode(String),
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        Error::Js(value.as_string().unwrap_or_else(|| format!("{value:?}")))
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Decode(e)
    }
}

fn decode_err(e: impl std::fmt::Display) -> Error {
    Error::Decode(e.to_string())
}

/// A connected CIP-30 API object - the result of `window.cardano.<name>.enable()`.
pub struct Cip30 {
    api: JsValue,
}

impl Cip30 {
    /// `name` is the extension's key on `window.cardano` (e.g. `"eternl"`, `"nami"`, `"lace"`).
    pub async fn connect(name: &str) -> Result<Self, Error> {
        let window = web_sys::window().expect("Cip30::connect called outside a browser");
        let cardano = js_sys::Reflect::get(&window, &JsValue::from_str("cardano"))?;
        let provider = js_sys::Reflect::get(&cardano, &JsValue::from_str(name))?;
        if provider.is_undefined() {
            return Err(Error::NotFound(name.to_string()));
        }

        let enable = js_sys::Reflect::get(&provider, &JsValue::from_str("enable"))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| Error::Js(format!("{name} has no enable()")))?;
        let api = JsFuture::from(js_sys::Promise::from(enable.call0(&provider)?)).await?;

        Ok(Self { api })
    }

    /// Call a CIP-30 method and await its promise.
    async fn call(&self, method: &str, args: &[JsValue]) -> Result<JsValue, Error> {
        let f = js_sys::Reflect::get(&self.api, &JsValue::from_str(method))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| Error::Js(format!("api.{method} is not a function")))?;
        let args = args.iter().cloned().collect::<js_sys::Array>();
        Ok(JsFuture::from(js_sys::Promise::from(f.apply(&self.api, &args)?)).await?)
    }

    /// As `call`, but expects the promise to resolve to a JS string.
    async fn call_str(&self, method: &str, args: &[JsValue]) -> Result<String, Error> {
        self.call(method, args)
            .await?
            .as_string()
            .ok_or_else(|| Error::Decode(format!("{method} did not return a string")))
    }
}

impl Wallet for Cip30 {
    type Error = Error;

    async fn network_id(&self) -> Result<NetworkId, Self::Error> {
        let id = self.call("getNetworkId", &[]).await?;
        let id = id
            .as_f64()
            .filter(|n| n.fract() == 0.0 && (0.0..=255.0).contains(n))
            .ok_or_else(|| Error::Decode("getNetworkId did not return a valid u8".into()))?;
        NetworkId::try_from(id as u8).map_err(decode_err)
    }

    async fn change_address(&self) -> Result<Address<kind::Any>, Self::Error> {
        self.call_str("getChangeAddress", &[])
            .await?
            .parse()
            .map_err(|e| Error::Decode(format!("parsing change address: {e}")))
    }

    async fn utxos(
        &self,
        value: Option<Value<u64>>,
    ) -> Result<Option<BTreeMap<Input, Output>>, Self::Error> {
        // Omitted `amount` returns every UTXO; given, `null` means the
        // wallet can't cover it.
        let args = match &value {
            Some(v) => vec![JsValue::from_str(&cbor::to_cbor_hex(v)?)],
            None => vec![],
        };
        let result = self.call("getUtxos", &args).await?;
        if result.is_null() {
            return Ok(None);
        }

        let hex_list: Vec<String> = serde_wasm_bindgen::from_value(result).map_err(decode_err)?;
        let utxos = hex_list
            .iter()
            .map(|hex| cbor::from_cbor_hex::<(Input, Output)>(hex).map_err(Error::from))
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        Ok((!utxos.is_empty()).then_some(utxos))
    }

    async fn sign_tx(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<(VerificationKey, Signature), Self::Error> {
        let args = [
            JsValue::from_str(&cbor::to_cbor_hex(tx)?),
            JsValue::from_bool(true),
        ]; // partialSign
        let witness_set_hex = self.call_str("signTx", &args).await?;

        // A partial transaction_witness_set, not a bare (key, signature) pair.
        let witness_set = cbor::from_cbor_hex::<WitnessSet>(&witness_set_hex)?;
        let witness = witness_set
            .vkeywitness
            .and_then(|w| w.into_iter().next())
            .ok_or_else(|| Error::Decode("witness set contains no vkey witnesses".into()))?;

        let vkey: [u8; VerificationKey::SIZE] = witness
            .vkey
            .try_into()
            .map_err(|_| Error::Decode("wrong verification-key length".into()))?;
        let signature: [u8; Signature::SIZE] = witness
            .signature
            .try_into()
            .map_err(|_| Error::Decode("wrong signature length".into()))?;

        Ok((VerificationKey::from(vkey), Signature::from(signature)))
    }

    async fn submit(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<Hash<32>, Self::Error> {
        let args = [JsValue::from_str(&cbor::to_cbor_hex(tx)?)];
        let hash_hex = self.call_str("submitTx", &args).await?;
        let bytes: [u8; 32] = hex::decode(&hash_hex)
            .map_err(decode_err)?
            .try_into()
            .map_err(|_| Error::Decode("submitTx returned the wrong hash length".into()))?;
        Ok(Hash::from(bytes))
    }
}

/// `transaction_witness_set` per the Conway CDDL, decoded only as far as
/// the vkey witnesses. `#[cbor(map)]` skips keys 1-7 rather than erroring.
///
/// TEMPORARY stand-in for a real witness-set type - delete once this
/// crate takes on a proper CBOR-primitives dependency (pallas or otherwise).
#[derive(Debug, Clone, minicbor::Decode)]
#[cbor(map)]
struct WitnessSet {
    #[n(0)]
    vkeywitness: Option<Vec<VKeyWitness>>,
}

#[derive(Debug, Clone, minicbor::Decode)]
struct VKeyWitness {
    #[n(0)]
    #[cbor(with = "minicbor::bytes")]
    vkey: Vec<u8>,
    #[n(1)]
    #[cbor(with = "minicbor::bytes")]
    signature: Vec<u8>,
}
