use cardano_tx_builder::{Address, SigningKey, VerificationKey, address::kind};
use konduit_tx::adaptor::AdaptorPreferences;

use crate::common::{self, ChannelParameters};

pub struct Config {
    pub wallet: SigningKey,
    pub channel_parameters: ChannelParameters,
    pub tx_preferences: AdaptorPreferences,
    pub host_address: Address<kind::Shelley>,
}

impl Config {
    pub fn from_args(common: common::Args, admin: super::Args) -> Self {
        let common::Args {
            signing_key: wallet,
            close_period,
            tag_length,
            host_address,
            ..
        } = common;
        let adaptor_key = VerificationKey::from(&wallet);
        let channel_parameters = ChannelParameters {
            adaptor_key,
            close_period,
            tag_length,
        };
        let tx_preferences = AdaptorPreferences {
            min_single: admin.min_single,
            min_total: admin.min_total,
        };
        Self {
            wallet,
            channel_parameters,
            tx_preferences,
            host_address,
        }
    }
}
