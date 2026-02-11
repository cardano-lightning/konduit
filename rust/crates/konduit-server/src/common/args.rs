use cardano_tx_builder::{Address, SigningKey, address::kind};
use konduit_data::Duration;

use crate::{common::metavar, env};

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
