use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen: String,
    pub sync_interval_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:2567".to_string(),
            sync_interval_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub inbound: super::inbounds::Config,
    pub outbound: crate::Config,
}

pub fn init_config(path: &PathBuf, force: bool) -> anyhow::Result<()> {
    let toml = toml::to_string_pretty(&Config::default())?;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true);
    if force {
        opts.create(true).truncate(true);
    } else {
        opts.create_new(true);
    }
    std::io::Write::write_all(
        &mut opts
            .open(path)
            .with_context(|| format!("opening {} (use --force)", path.display()))?,
        toml.as_bytes(),
    )?;
    log::info!("wrote default config to {}", path.display());
    Ok(())
}

pub fn load_config(path: &PathBuf) -> anyhow::Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading config file at {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing config file at {}", path.display()))
}
