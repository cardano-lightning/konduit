use anyhow::anyhow;
use cardano_tx_builder::Hash;
use clap::Parser;
use std::{collections::BTreeMap, sync::LazyLock};

mod cmd;
mod connector;
mod env;
mod metavar;

/// Get the validator blueprint at compile-time, and make the validator hash available on-demand.
pub(crate) static KONDUIT_VALIDATOR_HASH: LazyLock<Hash<28>> = LazyLock::new(|| {
    let blueprint: BTreeMap<String, serde_json::Value> = serde_json::from_str(include_str!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../../kernel/plutus.json",)
    ))
    .unwrap_or_else(|e| panic!("failed to parse blueprint: {e}"));

    blueprint
        .get("validators")
        .and_then(|value| value.as_array())
        .and_then(|validators| validators.first())
        .and_then(|value| value.as_object())
        .and_then(|validator| validator.get("hash"))
        .and_then(|value| value.as_str())
        .and_then(|s| Hash::try_from(s).ok())
        .unwrap_or_else(|| panic!("failed to extract validator's hash from blueprint"))
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().map_err(|e| anyhow!(e).context("fail to parse .env"))?;
    cmd::Cmd::parse().execute().await
}
