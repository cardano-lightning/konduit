use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub session: cardano_session::Config,
}

impl Default for Config {
    fn default() -> Self {
        let session = cardano_session::Config {
            tip_cache_path: "/tmp/konduit-session-tip.json".into(),
            addressbook_path: "/tmp/konduit-session-addressbook.json".into(),
            ..Default::default()
        };
        Self { session }
    }
}
