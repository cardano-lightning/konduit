use std::path::Path;

use anyhow::Context;

use crate::Config;

pub fn init(path: &Path, force: bool) -> anyhow::Result<()> {
    anyhow::ensure!(
        force || !path.exists(),
        "{} already exists - pass --force to overwrite",
        path.display()
    );
    let contents =
        toml::to_string_pretty(&Config::default()).context("serializing default config")?;
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    print_json(&serde_json::json!({ "wrote": path.display().to_string() }))
}

pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading config at {} - run `init` first?", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parsing config at {}", path.display()))
}

// Shared by the `wallet`, `addressbook`, `tip`, and `refresh` adapters too.
pub(super) fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
