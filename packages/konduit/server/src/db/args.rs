#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// The path to the database file(s)
    #[clap(long, default_value = "konduit.db", env = crate::env::DB_PATH)]
    pub db_path: String,
}

impl DbArgs {
    pub fn build(self) -> Result<super::Db, super::Error> {
        super::Db::open(&self.db_path)
    }
}
