use cardano_tx_builder::{Address, SigningKey, address::kind};
use clap::Args;
use konduit_data::Duration;

use crate::env;

impl AdminConfig {
    pub fn from_args(common: crate::CommonArgs, admin: super::Args) -> Self {
        let crate::CommonArgs {
            signing_key: wallet,
            close_period,
            tag_length,
            ..
        } = common;
        let key = VerificationKey::from(&wallet);
        let channel_parameters = ChannelParameters {
            key,
            close_period,
            tag_length,
        };
        let tx_preferences = AdaptorPreferences {
            min_single: config.min_single,
            min_total: min_total,
        };
        Self {
            wallet: config.signing_key,
            channel_parameters,
            tx_preferences,
            host_address,
        }
    }
}
