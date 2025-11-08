use cardano_connect::CardanoConnect;
use cardano_connect_wasm::{self as wasm, CardanoConnector};
use cardano_tx_builder::{
    Credential, Hash, Input, NetworkId, transaction::TransactionReadyForSigning,
};
use konduit_data::Tag;
use konduit_tx::{KONDUIT_VALIDATOR, close_one};
use std::{collections::BTreeMap, str::FromStr};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn close(
    // Cardano's connector
    connector: &CardanoConnector,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: &[u8],
    // Consumer's verification key, allowed to *add* funds.
    consumer: &[u8],
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: &[u8],
    // UTxO reference holding a deployed script.
    script_ref: &str,
) -> wasm::Result<TransactionReadyForSigning> {
    let consumer_verification_key = <[u8; 32]>::try_from(consumer)
        .expect("invalid verification key length")
        .into();

    let consumer_payment_credential =
        Credential::from_key(Hash::<28>::new(consumer_verification_key));

    let adaptor_verification_key = <[u8; 32]>::try_from(adaptor)
        .expect("invalid verification key length")
        .into();

    let tag = Tag::from(tag);

    let script_ref =
        Input::from_str(script_ref).expect("invalid transaction input as script reference");

    let mut consumer_utxos = connector
        .utxos_at(&consumer_payment_credential, None)
        .await?;

    let mut script_utxos = connector
        .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
        .await?;

    let mut utxos = BTreeMap::new();
    utxos.append(&mut consumer_utxos);
    utxos.append(&mut script_utxos);

    Ok(TransactionReadyForSigning::from(close_one(
        &utxos,
        &connector.protocol_parameters().await?,
        NetworkId::from(connector.network()),
        &script_ref,
        &tag,
        consumer_verification_key,
        adaptor_verification_key,
    )?))
}
