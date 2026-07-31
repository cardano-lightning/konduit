use clap::Parser;

#[cfg(feature = "cli")]
#[derive(Debug, clap::Parser)]
#[clap(
    version,
    about = "Konduit channel-state indexer: walks Kupo matches into the local SQLite DB.",
    long_about = None
)]
struct Args {
    /// Hostname or IP of the Kupo server.
    #[arg(long, env = "KUPO_HOST", default_value = "127.0.0.1")]
    pub host: String,

    /// TCP port of the Kupo server.
    #[arg(long, env = "KUPO_PORT", default_value_t = 1442)]
    pub port: u16,

    /// Path to the SQLite database file. Created if it doesn't exist.
    /// Ignored when `--in-memory` is set.
    #[arg(long, env = "INDEXER_DB_PATH", default_value = "konduit.sqlite3")]
    pub db_path: std::path::PathBuf,

    /// Use an in-memory SQLite database. Convenient for tests and
    /// single-pass smoke runs.
    #[arg(long, env = "INDEXER_IN_MEMORY", default_value_t = false)]
    pub in_memory: bool,

    /// Keep running passes in a loop. Without this flag the indexer
    /// performs exactly one pass and exits.
    #[arg(long, default_value_t = false)]
    pub loop_forever: bool,

    /// Number of seconds to wait between passes when `--loop` is set.
    #[arg(long, env = "INDEXER_DELAY_SECS", default_value_t = 5)]
    pub delay: u64,

    /// Maximum number of rollback-reconciliation retries before the
    /// indexer errors out and exits.
    #[arg(long, env = "INDEXER_MAX_RETRIES", default_value_t = 5)]
    pub max_retries: u32,
}

// Indexer {
//     fn new(kupo: kupo_client::blocking::Client, queries: &'a mut Q, max_retries: u32) -> Self {
#[cfg(feature = "cli")]
fn main() -> anyhow::Result<()> {
    use konduit_indexer::{indexer::Indexer, store::sqlite::SqliteStore};

    let args = Args::parse();

    println!(
        "indexer: host={}, port={}, db_path={:?}, in_memory={}, loop_forever={}, delay={}, max_retries={}",
        args.host,
        args.port,
        args.db_path,
        args.in_memory,
        args.loop_forever,
        args.delay,
        args.max_retries
    );

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
        use konduit_indexer::indexer::Config;

        let config = Config {
            max_sync_retries: args.max_retries,
        };
        Indexer::new(store, kupo, config)
    };

    // loop {
    indexer.sync()?;

    //    if !args.loop_forever {
    //        break;
    //    }
    //}

    Ok(())
}
