mod cardano;
mod cmd;
mod config;
mod connector;
mod env;
mod shared;
mod tip;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;
    cmd::Cmd::init()?.run().await
}
