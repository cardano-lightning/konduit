#![cfg(all(target_arch = "wasm32", feature = "web"))]
//! Manual browser test harness for the CIP-30 backend. Not automated -
//! `wasm-pack build --target web --features web`, open `www/index.html`,
//! drive it from devtools via `window.wallet.*`.

use crate::{Cip30, Wallet, cbor};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct Cip30Handle(Cip30);

#[wasm_bindgen(js_name = connect)]
pub async fn connect(name: String) -> Result<Cip30Handle, JsError> {
    Cip30::connect(&name).await.map(Cip30Handle).map_err(to_err)
}

#[wasm_bindgen(getter_with_clone)]
pub struct BuiltTx {
    pub tx_hex: String,
    pub tx_id: String,
}

#[wasm_bindgen]
impl Cip30Handle {
    #[wasm_bindgen(js_name = networkId)]
    pub async fn network_id(&self) -> Result<u8, JsError> {
        self.0.network_id().await.map(u8::from).map_err(to_err)
    }

    #[wasm_bindgen(js_name = changeAddress)]
    pub async fn change_address(&self) -> Result<String, JsError> {
        // `Address` has no `to_hex` - its Display (paired with the
        // `FromStr` already relied on in cip30.rs's change_address) is
        // the confirmed-working string form.
        self.0
            .change_address()
            .await
            .map(|a| a.to_string())
            .map_err(to_err)
    }

    /// Tx ids of the wallet's current UTXOs - a debug view for eyeballing
    /// against a submitted tx hash, not a real UTXO reader.
    #[wasm_bindgen(js_name = getUtxos)]
    pub async fn get_utxos(&self) -> Result<Vec<String>, JsError> {
        Ok(self
            .0
            .utxos(None)
            .await
            .map_err(to_err)?
            .into_iter()
            .flatten()
            .map(|(input, _)| hex::encode(input.transaction_id().as_ref()))
            .collect())
    }

    /// Sign `tx_hex` (a complete tx built externally - no builder here),
    /// attach the returned witness, submit.
    #[wasm_bindgen(js_name = signAndSubmit)]
    pub async fn sign_and_submit(&self, tx_hex: String) -> Result<String, JsError> {
        let mut tx: cardano_sdk::Transaction<cardano_sdk::transaction::state::ReadyForSigning> =
            cbor::from_cbor_hex(&tx_hex).map_err(to_err)?;
        let (vkey, sig) = self.0.sign_tx(&tx).await.map_err(to_err)?;
        tx.add_witness(vkey, sig); // ASSUMPTION: Transaction::add_witness(vkey, sig) -> &mut Self
        let hash = self.0.submit(&tx).await.map_err(to_err)?;
        Ok(hex::encode(hash.as_ref()))
    }

    /// Build a 1 ADA self-payment tx from this wallet's own network,
    /// UTXOs, and change address. CIP-30's `networkId` only distinguishes
    /// mainnet (1) from testnet (0) - any non-mainnet id uses preprod's
    /// parameters, since there's no way to tell preprod from preview
    /// through CIP-30 alone.
    #[wasm_bindgen(js_name = buildSelfPaymentTx)]
    pub async fn build_self_payment_tx(&self) -> Result<BuiltTx, JsError> {
        let network = if u8::from(self.0.network_id().await.map_err(to_err)?) == 1 {
            "mainnet"
        } else {
            "preprod"
        };
        let protocol_parameters =
            crate::txs::protocol_parameters(network).expect("valid network name");
        let address = self.0.change_address().await.map_err(to_err)?;
        let utxos = self
            .0
            .utxos(None)
            .await
            .map_err(to_err)?
            .ok_or_else(|| JsError::new("wallet has no UTXOs to spend"))?;
        let tx = crate::txs::self_payment(
            &protocol_parameters,
            &utxos,
            address,
            crate::txs::ONE_ADA_LOVELACE,
        )
        .map_err(to_err)?;
        Ok(BuiltTx {
            tx_hex: cbor::to_cbor_hex(&tx).map_err(to_err)?,
            tx_id: hex::encode(tx.id().as_ref()),
        })
    }
}

fn to_err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}
