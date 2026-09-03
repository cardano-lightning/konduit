mod error;
mod http;
mod ops;
mod providers;
mod tx;
mod wire;

pub use error::ApiError;
pub use http::{AppState, Limits, OPENAPI_YAML, app, serve};
pub use ops::OpsStore;
pub use providers::{DolosLedger, History, KoiosHistory, Ledger, SubmitResult, TxPresence};
pub use wire::{TransactionSummary, Utxo};

use cardano_connector_utxorpc::{Config, UtxoRpc, live_network};
use cardano_sdk::Network;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
    let live = live_network(&config.dolos_endpoint)
        .await
        .map_err(|_| anyhow::anyhow!("failed to connect to Dolos"))?;
    cardano_connector_utxorpc::ensure_network_matches(
        Network::Mainnet,
        live,
        &config.dolos_endpoint,
    )
    .map_err(|_| anyhow::anyhow!("Dolos network does not match mainnet"))?;
    let rpc = UtxoRpc::connect(Config::new(config.dolos_endpoint, Network::Mainnet))
        .await
        .map_err(|_| anyhow::anyhow!("failed to connect to Dolos"))?;
    let ledger = Arc::new(DolosLedger::new(rpc));
    ledger
        .ready()
        .await
        .map_err(|_| anyhow::anyhow!("Dolos is unavailable"))?;
    let history = Arc::new(KoiosHistory::new(config.koios_url)?);
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
            ops: Arc::new(ops),
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
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let ids = match ops.pending_ids().await {
            Ok(ids) => ids,
            Err(error) => {
                log::error!("reconcile pending_ids failed: {error}");
                continue;
            }
        };
        let (height, slot) = match ledger.tip().await {
            Ok(tip) => tip,
            Err(error) => {
                log::error!("reconcile tip failed: {error}");
                continue;
            }
        };
        for id in ids {
            match ops.get(id).await {
                Ok(Some(mut record)) => {
                    if let Err(error) = ops
                        .reconcile_one(ledger.as_ref(), id, &mut record, height, slot, true)
                        .await
                    {
                        log::error!("reconcile_one failed: {error}");
                    }
                }
                Ok(None) => {}
                Err(error) => log::error!("reconcile get failed: {error}"),
            }
        }
    }
}

#[cfg(test)]
mod tests;
