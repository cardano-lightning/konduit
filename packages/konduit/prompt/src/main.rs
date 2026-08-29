mod cli;
mod cmd;
mod config;
mod keyring;
mod known_keys;
mod prompt;
mod receipt;
mod tx;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::parse().run().await
}
