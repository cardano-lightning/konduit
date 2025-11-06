use std::{sync::Arc, time::Duration};

use clap::Parser;
use konduit_adaptor::{Cmd, FxInterface, Server, cron::cron};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenvy::dotenv().ok();

    let cmd = Cmd::parse();
    let server = Server::from_cmd(cmd.clone())
        .await
        .expect("Failed to parse cmd");

    let fx_data = server.fx();
    let fx = Arc::new(cmd.fx.build().expect("Failed to setup fx"));
    cron(
        fx_data,
        move || {
            let fx_clone = fx.clone();
            async move { fx_clone.as_ref().get().await.ok() }
        },
        Duration::from_secs(60),
    );

    server.run().await
}
