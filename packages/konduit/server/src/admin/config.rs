use crate::common;
use cardano_sdk::{Address, SigningKey, address::kind};
use konduit_tx::adaptor::AdaptorPreferences;

pub struct Config {
    pub wallet: SigningKey,
    pub tx_preferences: AdaptorPreferences,
    pub host_address: Address<kind::Shelley>,
}

impl Config {
    pub fn from_args(common: common::Args, admin: super::Args) -> Self {
        let common::Args {
            signing_key: wallet,
            host_address,
            ..
        } = common;
        let tx_preferences = AdaptorPreferences {
            min_single: admin.min_single,
            min_total: admin.min_total,
        };
        Self {
            wallet,
            tx_preferences,
            host_address,
        }
    }
}
