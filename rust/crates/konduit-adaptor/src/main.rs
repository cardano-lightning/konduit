use std::{sync::Arc, time::Duration};

use cardano_tx_builder::SigningKey;
use clap::Parser;
use konduit_adaptor::{Admin, Args, Server, cron::cron};
use konduit_adaptor::{Args, env};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenvy::dotenv().ok();

    let args = Args::parse();

    let app_state = AppState::new(info, db, bln, None, connector);
    let server = Server::from_cmd(args.clone())
        .await
        .expect("Failed to parse args");

    // Fire off fx updater
    let fx_data = server.fx();
    let fx = Arc::new(args.fx.build().expect("Failed to setup fx"));
    cron(
        move || {
            let fx = fx.clone();
            let fx_data = fx_data.clone();
            async move {
                let new_value = fx.as_ref().get().await.ok();
                let mut data_guard = fx_data.write().await;
                *data_guard = new_value;
                Some(())
            }
        },
        Duration::from_secs(15 * 60),
    );

    let admin = {
        let skey = {
            let skey_hex = std::env::var(env::ADAPTOR_SKEY)
                .unwrap_or_else(|_| panic!("missing {} environment variable", env::ADAPTOR_SKEY));
            let bytes = hex::decode(skey_hex).expect("failed to decode signing key from hex");
            SigningKey::try_from(bytes).expect("failed to create signing key from bytes")
        };
        Admin::new(app.connector(), app.db(), app.info(), skey)
            .await
            .expect("failed to create admin instance")
    };
    cron(
        move || {
            let admin = admin.clone();
            async move {
                // TODO: We should log and panic in here.
                let _ = admin.sync().await;
                Some(())
            }
        },
        Duration::from_secs(5 * 60),
    );
    server.run().await
}
