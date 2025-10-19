use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use std::collections::HashMap;

/// Prefix used to identify cardano connet related variables.
pub const PREFIX: &str = "cardano_";

/// Envvar labels
pub const VIA: &str = "via";

/// Envvar labels blockfrost
pub const BLOCKFROST_PROJECT_ID: &str = "blockfrost_project_id";

pub fn from_env(env: &HashMap<String, String>) -> anyhow::Result<impl CardanoConnect> {
    Config::from_env(env)?.connector()
}

pub enum Config {
    Blockfrost(String),
    NotYetImplemented,
}

impl Config {
    pub fn from_env(env: &HashMap<String, String>) -> anyhow::Result<Self> {
        let cardano_env: HashMap<String, String> = env
            .iter()
            .filter_map(|(k, v)| k.strip_prefix(PREFIX).map(|k| (k.to_string(), v.clone())))
            .collect();
        let via = cardano_env
            .get(VIA)
            .ok_or(anyhow!("CardanoConnect {VIA} is not set."))?
            .to_lowercase();
        if via == "blockfrost" {
            let project_id = cardano_env
                .get(BLOCKFROST_PROJECT_ID)
                .ok_or(anyhow!("CardanoConnet {BLOCKFROST_PROJECT_ID}"))?;
            Ok(Self::Blockfrost(project_id.to_string()))
        } else {
            Err(anyhow!("CardanoConnet via `{via}` not yet implemented"))
        }
    }

    pub fn connector(self) -> anyhow::Result<impl CardanoConnect> {
        new(self)
    }
}

// FIXME : A single installation should support multiple connectors.
// This looks to support just one.
#[cfg(feature = "blockfrost")]
pub(crate) fn new(connector_config: Config) -> anyhow::Result<impl CardanoConnect> {
    use cardano_connect_blockfrost::Blockfrost;
    if let Config::Blockfrost(project_id) = connector_config {
        Ok(Blockfrost::new(project_id))
    } else {
        Err(anyhow!("Expect blockfrost config"))
    }
}

#[cfg(not(feature = "blockfrost"))]
pub(crate) fn new(_config: Config) -> anyhow::Result<impl CardanoConnect> {
    Err(anyhow!(
        "no Cardano connector configured; did you forget to choose one when compiling?"
    ))
}
