use std::sync::Arc;

use clap::Parser;
use konduit_adaptor::{admin, args, info, server};
use tokio::{sync::RwLock, time::interval};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenvy::dotenv().ok();

    let args = args::Args::parse();

    // FX
    let fx_every = args.fx.every;
    let fx_config = fx_client::cli::Config::from_args(args.fx)
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve FX configuration from provided flags"))?;
    let fx_client = fx_config.build()?;
    let fx_init_state = fx_client.get().await?;
    let fx_state = Arc::new(RwLock::new(fx_init_state));
    let fx_state_clone = fx_state.clone();
    tokio::spawn(async move {
        let mut ticker = interval(fx_every);
        loop {
            ticker.tick().await;
            match fx_client.get().await {
                Ok(new_state) => {
                    let mut w = fx_state_clone.write().await;
                    *w = new_state;
                }
                Err(e) => eprintln!("Background FX update failed: {}", e),
            }
        }
    });

    // CARDANO
    let cardano = Arc::new(args.cardano.build().await?);

    // DB
    let db = Arc::new(args.db.build()?);

    // BLN
    let bln = bln_client::cli::Config::from_args(args.bln)
        .map_err(|s| anyhow::anyhow!(s))?
        .build()?;

    // ADMIN
    let admin_every = args.admin.admin_every.clone();
    let admin_config = admin::Config::from_args(args.common.clone(), args.admin);
    let admin = Arc::new(admin::Service::new(admin_config, cardano.clone(), db.clone()).await?);
    tokio::spawn(async move {
        let admin = Arc::clone(&admin);
        let mut ticker = interval(admin_every);
        loop {
            ticker.tick().await;
            match admin.sync().await {
                Ok(_) => log::info!("Admin sync ok"),
                Err(e) => log::error!("Admin sync failed: {}", e),
            }
        }
    });

    // INFO
    let info = Arc::new(info::Info::from_args(&args.common));

    let server_data = server::Data::new(bln, db, fx_state, info);
    let server = server::Service::new(args.server, server_data);
    server.run().await?;
    Ok(())
}
