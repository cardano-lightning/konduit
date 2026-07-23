#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// The path to the database file(s)
    #[clap(long, default_value = "konduit.db", env = crate::env::DB_PATH)]
    pub db_path: String,
}
