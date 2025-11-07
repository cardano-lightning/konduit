use crate::CardanoConnector;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Credential, Hash, NetworkId, transaction::TransactionReadyForSigning};
use konduit_data::{Duration, Tag};
use konduit_tx::KONDUIT_VALIDATOR;
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn open(
    // Cardano's connector
    connector: &CardanoConnector,
    // Quantity of Lovelace to deposit into the channel
    amount: u64,
    // Consumer's verification key, allowed to *add* funds.
    consumer: &[u8],
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: &[u8],
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: &str,
    // Minimum time from `close` to `elapse`.
    close_period: u64,
) -> crate::Result<TransactionReadyForSigning> {
    let consumer = <[u8; 32]>::try_from(consumer)
        .expect("invalid verification key length")
        .into();

    let adaptor = <[u8; 32]>::try_from(adaptor)
        .expect("invalid verification key length")
        .into();

    let tag = Tag::from_str(tag).expect("invalid tag");

    let close_period = Duration(std::time::Duration::from_secs(close_period));

    let consumer_credential = Credential::from_key(Hash::<28>::new(consumer));

    Ok(TransactionReadyForSigning::from(konduit_tx::open(
        &connector.utxos_at(&consumer_credential, None).await?,
        &connector.protocol_parameters().await?,
        NetworkId::from(connector.network()),
        KONDUIT_VALIDATOR.hash,
        amount,
        consumer,
        adaptor,
        tag,
        close_period,
    )?))
}
