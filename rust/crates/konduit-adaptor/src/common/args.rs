use cardano_tx_builder::{Address, SigningKey, address::kind};
use konduit_data::Duration;

use crate::{common::metavar, env};

#[derive(Debug, Clone, clap::Args)]
pub struct CommonArgs {
    #[arg(long, env = env::SIGNING_KEY)]
    pub signing_key: SigningKey,
    #[arg(long, env = env::CLOSE_PERIOD, value_name=metavar::DURATION, default_value="24h")]
    pub close_period: Duration,
    #[arg(long, env = env::TAG_LENGTH, default_value = "32")]
    pub tag_length: usize,
    #[arg(long, env = env::HOST_ADDRESS)]
    pub host_address: Address<kind::Shelley>,
    // Amount in channel currency (eg lovelace)
    #[arg(long, env = env::FEE, default_value = "1000")]
    pub fee: u64,
}
