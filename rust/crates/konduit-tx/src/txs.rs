use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{
    Address, ChangeStrategy, Credential, Hash, Input, NetworkId, Output, PlutusData, Transaction,
    Value, VerificationKey, transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Datum, Duration, Stage, Tag};

pub async fn open(
    // A backend to Cardano
    connector: impl CardanoConnect,
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
) -> anyhow::Result<Transaction<ReadyForSigning>> {
    let consumer_payment_credential = Credential::from_key(Hash::<28>::new(consumer));

    let resolved_inputs = connector
        .utxos_at(&consumer_payment_credential, None)
        .await?;

    let from: Input = resolved_inputs
        .iter()
        .find(|(_, output)| output.value().lovelace() > amount)
        .map(|(input, _)| input.clone())
        .ok_or(anyhow!(
            "no sufficiently large output found in consumer's wallet"
        ))?;

    let network_id = NetworkId::from(connector.network());

    let contract_address =
        Address::from(Address::new(network_id, Credential::from_script(validator)));

    let consumer_change_address =
        Address::from(Address::new(network_id, consumer_payment_credential));

    let datum = PlutusData::from(Datum {
        own_hash: validator,
        constants: Constants {
            tag,
            add_vkey: consumer,
            sub_vkey: adaptor,
            close_period,
        },
        stage: Stage::Opened(amount),
    });

    Transaction::build(
        &connector.protocol_parameters().await?,
        &resolved_inputs,
        |transaction| {
            transaction
                .with_inputs([(from.clone(), None)])
                .with_outputs([Output::new(contract_address.clone(), Value::new(amount))
                    .with_datum(datum.clone())])
                .with_change_strategy(ChangeStrategy::as_last_output(
                    consumer_change_address.clone(),
                ))
                .ok()
        },
    )
}
