use crate::ResolvedInputs;
use cardano_tx_builder::{
    self as builder, Address, ChangeStrategy, Credential, Hash, Input, NetworkId, Output,
    PlutusData, ProtocolParameters, Value, VerificationKey,
    transaction::TransactionReadyForSigning,
};
use konduit_data::{Constants, Datum, Duration, Stage, Tag};
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
            close_period: Duration(std::time::Duration::from_millis(close_period)),
        }
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        format!("{:#?}", self)
    }
}

#[wasm_bindgen]
pub fn open(
    cfg: &OpenConfig,
    protocol_parameters: &ProtocolParameters,
    network_id: &NetworkId,
    resolved_inputs: &ResolvedInputs,
    from: &Input,
) -> Result<TransactionReadyForSigning, String> {
    let consumer_payment_credential = Credential::from_key(Hash::<28>::new(cfg.consumer));

    let contract_address = Address::from(Address::new(
        *network_id,
        Credential::from_script(cfg.validator),
    ));

    let consumer_change_address =
        Address::from(Address::new(*network_id, consumer_payment_credential));

    let datum = PlutusData::from(Datum {
        own_hash: cfg.validator,
        constants: Constants {
            tag: cfg.tag.clone(),
            add_vkey: cfg.consumer,
            sub_vkey: cfg.adaptor,
            close_period: cfg.close_period,
        },
        stage: Stage::Opened(cfg.amount),
    });

    let resolved_inputs = resolved_inputs
        .iter()
        .map(|(i, o)| (i.clone(), o.clone()))
        .collect();

    Ok(TransactionReadyForSigning::from(
        builder::Transaction::build(protocol_parameters, &resolved_inputs, |transaction| {
            transaction
                .with_inputs([(from.clone(), None)])
                .with_outputs([
                    Output::new(contract_address.clone(), Value::new(cfg.amount))
                        .with_datum(datum.clone()),
                ])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    consumer_change_address.clone(),
                ))
                .ok()
        })
        .map_err(|e| e.to_string())?,
    ))
}
