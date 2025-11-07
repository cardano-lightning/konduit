use crate::{env, metavar};
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{self as cardano, SigningKey, VerificationKey};
use konduit_data as konduit;
use konduit_tx::{Lovelace, open};

/// Open a channel with an adaptor and deposit some funds into it.
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Quantity of Lovelace to deposit into the channel
    #[clap(long, value_name = metavar::LOVELACE)]
    amount: Lovelace,

    /// We also assume that the consumer is opening that channel and paying for it.
    #[clap(
        long,
        value_name = metavar::ED25519_SIGNING_KEY,
        env = env::WALLET_SIGNING_KEY
    )]
    consumer_signing_key: SigningKey,

    /// Adaptor's verification key, allowed to *sub* funds
    #[clap(long, value_name = metavar::ED25519_VERIFICATION_KEY, env = env::ADAPTOR)]
    adaptor_verification_key: cardano::VerificationKey,

    /// An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: konduit::Tag,

    /// Minimum time from `close` to `elapse`. You may specify the duration with a unit; for
    /// examples: 5s, 27min, 3h
    #[clap(long, value_name = metavar::DURATION, env = env::CLOSE_PERIOD)]
    close_period: konduit::Duration,

    #[arg(short = 'd', long)]
    dry_run: bool,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let consumer_verification_key = VerificationKey::from(&self.consumer_signing_key);
        let consumer_payment_credential = cardano::Credential::from_key(cardano::Hash::<28>::new(
            consumer_verification_key,
        ));
        let utxos = connector
            .utxos_at(&consumer_payment_credential, None)
            .await?;
        let protocol_parameters = connector.protocol_parameters().await?;
        let network_id = cardano::NetworkId::from(connector.network());

        let mut open_transaction = open(
            &utxos,
            &protocol_parameters,
            network_id,
            crate::KONDUIT_VALIDATOR.hash,
            self.amount,
            consumer_verification_key,
            self.adaptor_verification_key,
            self.channel_tag,
            self.close_period,
        )?;
        open_transaction.sign(self.consumer_signing_key);
        if !self.dry_run {
            let tx_id = connector.submit(&open_transaction).await?;
            println!("Transaction submitted with ID: {}", tx_id);
        }

        println!("{open_transaction}");

        Ok(())
    }
}
