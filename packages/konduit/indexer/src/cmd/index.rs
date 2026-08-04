use konduit_indexer::indexer::Config;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
#[clap(
    about = "Run a sync pass against Kupo and persist indexed state to the SQLite store.",
    long_about = None
)]
pub struct Args {
    #[arg(long, env = "KUPO_HOST", default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, env = "KUPO_PORT", default_value_t = 1442)]
    pub port: u16,
    /// Path to the SQLite database file. Created if it doesn't exist.
    /// Ignored when `--in-memory` is set.
    #[arg(long, env = "INDEXER_DB_PATH", default_value = "konduit.sqlite3")]
    pub db_path: PathBuf,
    /// Use an in-memory SQLite database. Convenient for tests and
    /// single-pass smoke runs.
    #[arg(long, env = "INDEXER_IN_MEMORY", default_value_t = false)]
    pub in_memory: bool,
    /// Maximum number of rollback-reconciliation retries before the
    /// indexer errors out and exits.
    #[arg(long, env = "INDEXER_MAX_RETRIES", default_value_t = 5)]
    pub max_retries: u32,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    use konduit_indexer::{indexer::Indexer, store::sqlite::SqliteStore};
    let base_url = format!("http://{}:{}", args.host, args.port);
    let kupo = kupo_client::blocking::Client::new(&base_url)?;
    let store = {
        let conn = if args.in_memory {
            rusqlite::Connection::open_in_memory()?
        } else {
            rusqlite::Connection::open(&args.db_path)?
        };
        SqliteStore::new(conn)
    }?;
    let mut indexer = {
        let config = Config {
            max_sync_retries: args.max_retries,
        };
        Indexer::new(store, kupo, config)
    };
    indexer.sync()?;
    Ok(())
}
