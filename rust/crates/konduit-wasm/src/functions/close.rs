use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_connect_wasm::{self as wasm, CardanoConnector};
use cardano_tx_builder::{Credential, Hash, NetworkId, transaction::TransactionReadyForSigning};
use konduit_data::Tag;
use konduit_tx::{
    Bounds, KONDUIT_VALIDATOR, NetworkParameters,
    consumer::{self, Intent},
};
use std::{collections::BTreeMap, ops::Deref};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn close(
    // Cardano's connector
    connector: &CardanoConnector,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: &[u8],
    // Consumer's verification key, allowed to *add* funds.
    consumer: &[u8],
) -> wasm::Result<TransactionReadyForSigning> {
    let consumer_verification_key = <[u8; 32]>::try_from(consumer)
        .map_err(|_| anyhow!("invalid verification key length"))?
        .into();

    let consumer_payment_credential =
        Credential::from_key(Hash::<28>::new(consumer_verification_key));

    let tag = Tag::from(tag);

    let mut consumer_utxos = connector
        .utxos_at(&consumer_payment_credential, None)
        .await?;

    let mut script_utxos = connector
        .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
        .await?;

    let mut utxos = BTreeMap::new();
    utxos.append(&mut consumer_utxos);
    utxos.append(&mut script_utxos);

    let network_parameters = NetworkParameters {
        network_id: NetworkId::from(*connector.network().deref()),
        protocol_parameters: connector.protocol_parameters().await?,
    };

    Ok(TransactionReadyForSigning::from(consumer::tx(
        &network_parameters,
        &consumer_verification_key,
        vec![],
        BTreeMap::from([(tag, Intent::Close)]),
        &utxos,
        Bounds::twenty_mins(),
    )?))
}
