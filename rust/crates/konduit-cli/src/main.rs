use anyhow::anyhow;
use clap::Parser;

mod cmd;
mod connector;
mod env;
mod metavar;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().map_err(|e| anyhow!(e).context("fail to parse .env"))?;
    cmd::Cmd::parse().execute().await
}
