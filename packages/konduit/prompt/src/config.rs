use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::keyring;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub session: cardano_session::Config,
    pub keyring: keyring::Config,
    pub known_keys: BTreeMap<String, konduit_data::VerifyingKey>,
}

impl Default for Config {
    fn default() -> Self {
        #[allow(unused)]
        let mut session = cardano_session::Config::default();

        Self {
            session,
            keyring: Default::default(),
            known_keys: Default::default(),
        }
    }
}
