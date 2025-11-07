use crate::CardanoConnector;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{
    Credential, Hash, NetworkId, VerificationKey, transaction::TransactionReadyForSigning,
};
use konduit_data::{Duration, Tag};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug)]
pub struct OpenConfig {
    // Konduit's validator hash,
    validator: Hash<28>,
    // Quantity of Lovelace to deposit into the channel
    amount: u64,
    // Consumer's verification key, allowed to *add* funds.
    consumer: VerificationKey,
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: VerificationKey,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: Tag,
    // Minimum time from `close` to `elapse`.
    close_period: Duration,
}

#[wasm_bindgen]
impl OpenConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        validator: Vec<u8>,
        amount: u64,
        consumer: Vec<u8>,
        adaptor: Vec<u8>,
        tag: String,
        close_period: u64,
    ) -> Self {
        OpenConfig {
            validator: <[u8; 28]>::try_from(validator)
                .expect("invalid validator hash length")
                .into(),
            amount,
            consumer: <[u8; 32]>::try_from(consumer)
                .expect("invalid verification key length")
                .into(),
            adaptor: <[u8; 32]>::try_from(adaptor)
                .expect("invalid verification key length")
                .into(),
            tag: Tag::from_str(&tag).expect("invalid tag"),
            close_period: Duration(std::time::Duration::from_secs(close_period)),
        }
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[wasm_bindgen]
pub async fn open(
    connector: &CardanoConnector,
    cfg: &OpenConfig,
) -> crate::Result<TransactionReadyForSigning> {
    let consumer_credential = Credential::from_key(Hash::<28>::new(cfg.consumer));

    Ok(TransactionReadyForSigning::from(konduit_tx::open(
        &connector.utxos_at(&consumer_credential, None).await?,
        &connector.protocol_parameters().await?,
        NetworkId::from(connector.network()),
        cfg.validator,
        cfg.amount,
        cfg.consumer,
        cfg.adaptor,
        cfg.tag.clone(),
        cfg.close_period,
    )?))
}
