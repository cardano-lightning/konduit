use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_connect_wasm::{self as wasm, CardanoConnector};
use cardano_tx_builder::{Credential, Hash, NetworkId, transaction::TransactionReadyForSigning};
use konduit_data::{Duration, Tag};
use konduit_tx::{
    Bounds, NetworkParameters,
    consumer::{self, OpenIntent},
};
use std::{collections::btree_map::BTreeMap, ops::Deref};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn open(
    // Cardano's connector
    connector: &CardanoConnector,
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    tag: &[u8],
    // Consumer's verification key, allowed to *add* funds.
    consumer: &[u8],
    // Adaptor's verification key, allowed to *sub* funds
    adaptor: &[u8],
    // Minimum time from `close` to `elapse`.
    close_period: u64,
    // Quantity of Lovelace to deposit into the channel
    amount: u64,
) -> wasm::Result<TransactionReadyForSigning> {
    let consumer = <[u8; 32]>::try_from(consumer)
        .map_err(|_| anyhow!("invalid verification key length"))?
        .into();

    let adaptor = <[u8; 32]>::try_from(adaptor)
        .map_err(|_| anyhow!("invalid verification key length"))?
        .into();

    let tag = Tag::from(tag);

    let close_period = Duration(std::time::Duration::from_secs(close_period));

    let consumer_credential = Credential::from_key(Hash::<28>::new(consumer));

    let network_parameters = NetworkParameters {
        network_id: NetworkId::from(*connector.network().deref()),
        protocol_parameters: connector.protocol_parameters().await?,
    };

    Ok(TransactionReadyForSigning::from(consumer::tx(
        &network_parameters,
        &consumer,
        vec![OpenIntent {
            tag,
            sub_vkey: adaptor,
            close_period,
            amount,
        }],
        BTreeMap::new(),
        &connector.utxos_at(&consumer_credential, None).await?,
        Bounds::twenty_mins(),
    )?))
}
