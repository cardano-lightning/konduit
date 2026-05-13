#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// Run a mock db (clap cannot handle name clashes)
    #[arg(long("db-mock"), default_value_t = false)]
    pub db_mock: bool,
    /// Db with sled
    #[clap(flatten)]
    pub sled: Option<super::with_sled::Args>,
}

impl DbArgs {
    pub fn build(self) -> super::Result<impl super::Api> {
        if self.db_mock {
            let db = super::with_sled::WithSled::open_temporary().expect("failed to open mock db");
            Ok(db)
        } else if let Some(args) = &self.sled {
            let db = super::with_sled::WithSled::try_from(args).expect("failed to open db");
            Ok(db)
        } else {
            panic!("db failed to init")
        }
    }
}
