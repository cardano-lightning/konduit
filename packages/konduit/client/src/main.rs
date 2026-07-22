use clap::Parser;
use konduit_client::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    if std::fs::exists(".env.consumer")? {
        dotenvy::from_filename(".env.consumer").map_err(|err| {
            anyhow::anyhow!("{err}").context("failed to load adaptor-specific environment")
        })?;
    }

    Cli::parse().run().await
}
