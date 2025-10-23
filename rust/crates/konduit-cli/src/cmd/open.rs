use crate::{env, metavar};
use cardano_connect::CardanoConnect;
use cardano_tx_builder as cardano;
use konduit_data as konduit;
use konduit_tx::open;

/// Open a channel with an adaptor and deposit some funds into it.
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Quantity of Lovelace to deposit into the channel
    #[clap(long, value_name = metavar::LOVELACE)]
    amount: u64,

    /// Consumer's verification key, allowed to *add* funds.
    ///
    /// We also assume that the consumer is opening that channel and paying for it.
    #[clap(long, value_name = metavar::ED25519_VERIFICATION_KEY, env = env::CONSUMER)]
    consumer: cardano::VerificationKey,

    /// Adaptor's verification key, allowed to *sub* funds
    #[clap(long, value_name = metavar::ED25519_VERIFICATION_KEY, env = env::ADAPTOR)]
    adaptor: cardano::VerificationKey,

    /// An (ideally) unique tag to discriminate channels and allow reuse of keys between them.
    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: konduit::Tag,

    /// Minimum time from `close` to `elapse`. You may specify the duration with a unit; for
    /// examples: 5s, 27min, 3h
    #[clap(long, value_name = metavar::DURATION, env = env::CLOSE_PERIOD)]
    close_period: konduit::Duration,
}

impl Args {
    pub(crate) async fn execute(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
        let open_transaction = open(
            connector,
            *crate::KONDUIT_VALIDATOR_HASH,
            self.amount,
            self.consumer,
            self.adaptor,
            self.channel_tag,
            self.close_period,
        )
        .await?;

        println!("{open_transaction}");

        Ok(())
    }
}
