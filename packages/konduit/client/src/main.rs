use clap::Parser;
use konduit_client::cli::Cli;

/// Load env if exists
fn load_env(path: &str) -> anyhow::Result<()> {
    if std::fs::exists(path)? {
        dotenvy::from_filename(path)
            .map_err(|err| anyhow::anyhow!("{err}").context("failed to load env"))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_env(".env.consumer")?;
    load_env(".env")?;
    env_logger::init();
    Cli::parse().run().await
}
