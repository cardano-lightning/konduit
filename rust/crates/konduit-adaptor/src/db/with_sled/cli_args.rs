#[derive(Debug, Clone, clap::Args)]
pub struct Args {
    /// The path to the database file
    #[clap(long, default_value = "konduit.db", env = crate::env::DB_PATH)]
    pub path: String,
}
