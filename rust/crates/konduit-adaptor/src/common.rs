use cardano_tx_builder::{Address, SigningKey, VerificationKey, address::kind};
use konduit_data::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::env;

/// These variables are either those used by more than one component,
/// or are mandatory.
/// These are not all variables required: component specific ones
/// are colocated with the component.
#[derive(Debug, Clone, clap::Args)]
pub struct CommonArgs {
    #[arg(long, env = env::SIGNING_KEY)]
    pub signing_key: SigningKey,
    #[arg(long, env = env::CLOSE_PERIOD, value_name=crate::metavar::DURATION, default_value="24h")]
    pub close_period: Duration,
    #[arg(long, env = env::TAG_LENGTH, default_value = "32")]
    pub tag_length: usize,
    #[arg(long, env = env::HOST_ADDRESS)]
    pub host_address: Address<kind::Shelley>,
    // Amount in channel currency (eg lovelace)
    #[arg(long, env = env::FEE, default_value = "1000")]
    pub fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelParameters {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub adaptor_key: VerificationKey,
    pub close_period: Duration,
    pub tag_length: usize,
}

impl ChannelParameters {
    pub fn from_args(args: CommonArgs) -> Self {
        let CommonArgs {
            signing_key,
            close_period,
            tag_length,
            ..
        } = args;
        let adaptor_key = VerificationKey::from(&signing_key);
        Self {
            adaptor_key,
            close_period,
            tag_length,
        }
    }
}
