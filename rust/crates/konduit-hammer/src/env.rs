use std::{
    fs,
    path::{Path, PathBuf},
};

/// A dot env file accompanies a config.toml.
/// The name is derived from the config, for example:
///
/// ```
/// preview.toml
/// .env.preview
/// ```

pub fn transform_path(config_path: &impl AsRef<Path>) -> PathBuf {
    let p = config_path.as_ref();
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    p.with_file_name(format!(".env.{}", stem))
}

pub fn load(config_path: &impl AsRef<Path>) -> anyhow::Result<()> {
    let dotenv_path = transform_path(config_path);
    if dotenv_path.exists() {
        dotenvy::from_path(&dotenv_path)?;
    }
    Ok(())
}

pub fn save(config_path: &impl AsRef<Path>, env_list: Vec<String>) -> anyhow::Result<()> {
    if !env_list.is_empty() {
        let dotenv_path = transform_path(config_path);
        fs::write(dotenv_path, env_list.join("\n") + "\n")?;
    }
    Ok(())
}
