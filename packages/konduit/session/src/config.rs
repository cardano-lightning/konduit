use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub session: cardano_session::Config,
}

impl Default for Config {
    fn default() -> Self {
        #[allow(unused)]
        let mut session = cardano_session::Config::default();
        // FIXME :: change session.tip_cache_path = "/tmp/...";
        // FIXME :: change session.addressbook_path = /tmp/...;

        Self { session }
    }
}
