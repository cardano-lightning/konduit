use clap::Parser;
<<<<<<< HEAD
use std::sync::Arc;

use konduit_adaptor::db::{DbInterface, open};
use konduit_adaptor::server::{init_on_new, run};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, env = "DB_PATH", default_value = "konduit.db")]
    pub db_path: String,
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = "PORT", default_value = "4444")]
    pub port: u16,
}
=======

use konduit_adaptor::{Cmd, Server};
>>>>>>> e3cb13e (Updates to konduit data.)

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenvy::dotenv().ok();
<<<<<<< HEAD
    let cli = Cli::parse();
    let db = open(cli.db_path).expect("Failed to open database");
    init_on_new(&db).await?;
    let db: Arc<dyn DbInterface + Send + Sync> = Arc::new(db);
    let bind_address = format!("{}:{}", cli.host, cli.port);
    run(db, bind_address).await
}
=======
    let cmd = Cmd::parse();
    println!("hello! {:?}", cmd);
    Server::from_cmd(cmd)
        .await
        .expect("Failed to parse cmd")
        .run()
        .await
}

// async fn cron_fx(period_seconds: u64, fx_data: Arc<RwLock<Option<Fx>>>) {
//     let mut interval = interval(Duration::from_secs(period_seconds));
//     loop {
//         interval.tick().await;
//         let new_data = get_fx(BaseCurrency::Eur).await.ok();
//         log::info!("FX CALLED");
//         let mut data_guard = fx_data.write().await;
//         *data_guard = new_data;
//     }
// }
>>>>>>> e3cb13e (Updates to konduit data.)
