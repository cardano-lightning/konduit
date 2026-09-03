use actix_web::web::Data;
use cardano_connector_server_utxorpc::{ServerConfig, boot, reconcile_loop, serve};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long, env = "DOLOS_ENDPOINT")]
    dolos_endpoint: String,
    #[arg(
        long,
        env = "KOIOS_URL",
        default_value = "https://api.koios.rest/api/v1"
    )]
    koios_url: String,
    #[arg(long, env = "KOIOS_API_KEY", hide_env_values = true)]
    koios_api_key: Option<String>,
    #[arg(long, env = "CONNECTOR_DB", default_value = "connector-ops.redb")]
    db_path: PathBuf,
    #[arg(long, env = "CONNECTOR_BIND", default_value = "127.0.0.1:8787")]
    bind: String,
    #[arg(long, default_value_t = 10_000)]
    max_pending: usize,
    #[arg(long, default_value_t = 32)]
    max_inflight: usize,
    #[arg(long, default_value_t = 30)]
    rate_per_minute: usize,
    #[arg(long, default_value_t = 1_000_000_000)]
    db_max_bytes: u64,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let args = Args::parse();
    let (state, bind) = boot(ServerConfig {
        dolos_endpoint: args.dolos_endpoint,
        koios_url: args.koios_url,
        koios_api_key: args.koios_api_key,
        db_path: args.db_path,
        bind: args.bind,
        max_pending: args.max_pending,
        max_inflight: args.max_inflight,
        rate_per_minute: args.rate_per_minute,
        db_max_bytes: args.db_max_bytes,
    })
    .await?;
    log::info!("binding {bind}");
    tokio::spawn(reconcile_loop(state.ledger.clone(), state.ops.clone()));
    serve(&bind, Data::new(state)).await?;
    Ok(())
}
