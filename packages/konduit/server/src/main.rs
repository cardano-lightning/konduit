use clap::Parser;
use konduit_server::{admin, args, index, server};
use konduit_tmp::AdaptorInfo;
use konduit_tx::InsufficientTotalGain;
use std::sync::Arc;
use tokio::{sync::RwLock, time::interval};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    dotenvy::dotenv().ok();
    if std::fs::exists(".env.adaptor")? {
        dotenvy::from_filename(".env.adaptor").map_err(|err| {
            anyhow::anyhow!("{err}").context("failed to load adaptor-specific environment")
        })?;
    }

    let args = args::Args::parse();

    // FX
    let fx_every = args.fx.every;
    let fx_config = fx_client::cli::Config::from_args(args.fx)
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve FX configuration from provided flags"))?;

    let fx_client = fx_config.build()?;
    let fx_init_state = fx_client.get().await?;
    let fx_state = Arc::new(RwLock::new(fx_init_state));
    let fx_state_clone = fx_state.clone();

    if !fx_every.is_zero() {
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
    }

    // CARDANO
    let cardano = Arc::new(args.cardano.build().await?);

    // DB
    let db = Arc::new(args.db.build()?);

    // BLN
    let bln = bln_client::cli::Config::from_args(args.bln)
        .map_err(|s| anyhow::anyhow!(s))?
        .build()?;

    let info = Arc::new(AdaptorInfo::from(args.common.clone()));

    // ADMIN :: Index
    let index_every = args.admin.admin_every;
    let index = Arc::new(index::Index::new(
        cardano.clone(),
        db.clone(),
        info.channel_parameters.clone(),
    ));
    let sync_index = Arc::clone(&index);

    actix_web::rt::spawn(async move {
        let mut ticker = interval(index_every);
        loop {
            ticker.tick().await;
            match sync_index.sync().await {
                Ok(()) => log::info!("Index sync ok"),
                Err(e) => log::error!("Index sync failed: {e:#}"),
            }
        }
    });

    // ADMIN :: Tx
    let admin_every = args.admin.admin_every;
    let admin_config = admin::Config::from_args(args.common.clone(), args.admin);
    let admin = Arc::new(
        admin::Service::new(admin_config, bln.clone(), cardano.clone(), db.clone()).await?,
    );
    let admin_for_sync = Arc::clone(&admin);

    actix_web::rt::spawn(async move {
        let admin = Arc::clone(&admin_for_sync);
        let mut ticker = interval(admin_every);
        loop {
            ticker.tick().await;
            match admin.sync().await {
                Ok(_) => log::info!("Admin sync ok"),
                Err(e) => {
                    if let Some(low_gain) = e.downcast_ref::<InsufficientTotalGain>() {
                        if low_gain.gain > 0 {
                            log::info!("Admin sync skipped: {}", low_gain);
                        }
                    } else {
                        log::error!("Admin sync failed: {e:#}");
                    }
                }
            }
        }
    });

    // INFO
    let server_data = server::Data::new(bln, db, fx_state, info, admin);
    let server = server::Service::new(args.server, server_data);

    server.run().await?;

    Ok(())
}
