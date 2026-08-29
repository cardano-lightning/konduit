//! NEW — `cli.rs` referenced `cmd::init`/`cmd::load_config` but their
//! implementation was never shown to me. This is a straightforward
//! TOML-file read/write, not ported from anything real; swap it out if
//! the actual project already has equivalent helpers (e.g. alongside
//! `cardano_session::cli::cmd`'s `load_tip`/`save_tip`/addressbook
//! functions, which this deliberately mirrors the shape of).

use std::path::Path;

use anyhow::{Context, Result};

use crate::config::Config;

/// Writes a default config to `path`. Errors if it already exists unless
/// `force`.
pub fn init(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!(
            "{} already exists (pass --force to overwrite)",
            path.display()
        );
    }
    let config = Config::default();
    let toml = toml::to_string_pretty(&config).context("serializing default config")?;
    std::fs::write(path, toml).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn load_config(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {} (run `init` first?)", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}
