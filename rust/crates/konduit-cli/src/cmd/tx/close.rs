use crate::{env, metavar};
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{self as cardano, Credential, Input, SigningKey, VerificationKey};
use konduit_data::{self as konduit};
use konduit_tx::{KONDUIT_VALIDATOR, close_one};
use std::collections::BTreeMap;

#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    // An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: konduit::Tag,

    // We also assume that the consumer is opening that channel and paying for it.
    #[clap(long, value_name = metavar::ED25519_SIGNING_KEY, env = env::WALLET_SIGNING_KEY)]
    consumer_signing_key: SigningKey,

    // We also assume that the consumer is opening that channel and paying for it.
    #[clap(long, value_name = metavar::ED25519_SIGNING_KEY, env = env::ADAPTOR)]
    adaptor_verification_key: VerificationKey,

    // If ommited then we assume that the reference script is published at the adaptor's address
    #[clap(long, value_name = metavar::OUTPUT_REF, env = env::SCRIPT_REF)]
    script_ref: Input,

    #[arg(long, short = 'd', long)]
    dry_run: bool,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let consumer_verification_key = VerificationKey::from(&self.consumer_signing_key);

        let consumer_payment_credential =
            cardano::Credential::from_key(cardano::Hash::<28>::new(consumer_verification_key));

        let mut consumer_utxos = connector
            .utxos_at(&consumer_payment_credential, None)
            .await?;

        let mut script_utxos = connector
            .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
            .await?;

        let mut utxos = BTreeMap::new();
        utxos.append(&mut consumer_utxos);
        utxos.append(&mut script_utxos);

        let protocol_parameters = connector.protocol_parameters().await?;

        let network_id = cardano::NetworkId::from(connector.network());

        let mut transaction = close_one(
            &utxos,
            &protocol_parameters,
            network_id,
            &self.script_ref,
            &self.channel_tag,
            consumer_verification_key,
            self.adaptor_verification_key,
        )?;

        transaction.sign(self.consumer_signing_key);

        eprintln!("{transaction}");

        if !self.dry_run {
            connector.submit(&transaction).await?;
            eprintln!("transaction successfully submitted");
            println!("{}", transaction.id());
        }

        Ok(())
    }
}
