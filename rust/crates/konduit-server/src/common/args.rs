use crate::{common::metavar, env};
use cardano_sdk::{Address, SigningKey, VerificationKey, address::kind};
use konduit_data::{AdaptorInfo, ChannelParameters, Duration, TosInfo, TxHelp};
use konduit_tx::KONDUIT_VALIDATOR;

#[derive(Debug, Clone, clap::Args)]
pub struct CommonArgs {
    /// Adaptors signing key.
    /// Used for both channel params (verification key of)
    /// and the wallet to administrate channels
    #[arg(long, env = env::SIGNING_KEY, hide_env_values = true)]
    pub signing_key: SigningKey,
    /// (Min) close period of channels.
    #[arg(long, env = env::CLOSE_PERIOD, value_name=metavar::DURATION, default_value="24h")]
    pub close_period: Duration,
    /// (Max) tag length (to prevent annoyingly long channel tags)
    #[arg(long, env = env::TAG_LENGTH, default_value = "32")]
    pub tag_length: usize,
    /// The host address of the konduit script to be referenced by txs
    #[arg(long, env = env::HOST_ADDRESS)]
    pub host_address: Address<kind::Shelley>,
    // Amount in channel currency (eg lovelace)
    #[arg(long, env = env::FEE, default_value = "1000")]
    pub fee: u64,
}

impl From<CommonArgs> for ChannelParameters {
    fn from(args: CommonArgs) -> Self {
        let adaptor_key = VerificationKey::from(&args.signing_key);
        Self {
            adaptor_key,
            close_period: args.close_period,
            tag_length: args.tag_length,
        }
    }
}

impl From<CommonArgs> for AdaptorInfo {
    fn from(args: CommonArgs) -> Self {
        let tos = TosInfo { flat_fee: args.fee };

        let tx_help = TxHelp {
            host_address: args.host_address.clone(),
            validator: KONDUIT_VALIDATOR.hash,
        };

        let channel_parameters = args.into();

        Self {
            tos,
            tx_help,
            channel_parameters,
        }
    }
}
