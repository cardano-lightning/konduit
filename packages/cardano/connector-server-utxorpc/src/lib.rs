mod error;
mod http;
mod ops;
mod providers;
mod tx;
mod wire;

pub use error::ApiError;
pub use http::{AppState, Limits, OPENAPI_YAML, app, serve};
pub use ops::OpsStore;
pub use providers::{DolosLedger, History, KoiosHistory, Ledger};

use cardano_connector_utxorpc::{Config, UtxoRpc, live_network};
use cardano_sdk::Network;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub struct ServerConfig {
    pub dolos_endpoint: String,
    pub koios_url: String,
    pub db_path: PathBuf,
    pub bind: String,
    pub max_pending: usize,
    pub max_inflight: usize,
    pub rate_per_minute: usize,
    pub db_max_bytes: u64,
}

pub async fn boot(
    config: ServerConfig,
) -> anyhow::Result<(AppState<DolosLedger, KoiosHistory>, String)> {
    let live = live_network(&config.dolos_endpoint).await?;
    cardano_connector_utxorpc::ensure_network_matches(
        Network::Mainnet,
        live,
        &config.dolos_endpoint,
    )?;
    let rpc = UtxoRpc::connect(Config::new(config.dolos_endpoint, Network::Mainnet)).await?;
    let ledger = DolosLedger::new(rpc);
    ledger
        .ready()
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let history = KoiosHistory::new(config.koios_url)?;
    let ops = OpsStore::open(
        &config.db_path,
        config.max_pending,
        config.db_max_bytes,
        config.max_inflight,
    )?;
    Ok((
        AppState {
            ledger,
            history,
            ops,
            limits: Limits {
                rate_per_minute: config.rate_per_minute,
            },
            hits: Mutex::new(Default::default()),
        },
        config.bind,
    ))
}

pub async fn reconcile_loop<L: Ledger>(ledger: std::sync::Arc<L>, ops: std::sync::Arc<OpsStore>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let Ok(ids) = ops.pending_ids() else {
            continue;
        };
        let Ok((height, slot)) = ledger.tip().await else {
            continue;
        };
        for id in ids {
            if let Ok(Some(mut record)) = ops.get(&id) {
                let _ = ops
                    .reconcile_one(ledger.as_ref(), &id, &mut record, height, slot, true)
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests;
