mod error;
pub use error::*;
mod api;
pub use api::Api;

// Helpers
mod coiter_with_default;

// Impls
mod with_sled;

#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// Db with sled
    #[clap(flatten)]
    pub sled: Option<with_sled::Args>,
}

impl DbArgs {
    pub fn build(self) -> error::Result<impl Api> {
        if let Some(args) = &self.sled {
            let db = with_sled::WithSled::try_from(args).expect("oops");
            Ok(db)
        } else {
            panic!("db failed to init")
        }
    }
}
