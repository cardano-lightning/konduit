mod cardano;
mod cmd;
mod config;
mod connector;
mod env;
mod output;
mod shared;
mod tip;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cmd::Cmd::init()?.run().await
}
