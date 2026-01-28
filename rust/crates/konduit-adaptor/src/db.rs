use thiserror::Error;

mod interface;
pub use interface::DbInterface;

mod coiter_with_default;

mod error;
mod with_sled;
pub use error::*;

#[derive(Debug, Error)]
pub enum DbInitError {
    #[error("Sled database error: {0}")]
    Sled(#[from] sled::Error),
    #[error("No database specified")]
    None,
}

#[derive(Debug, Clone, clap::Args)]
pub struct DbArgs {
    /// Db with sled
    #[clap(flatten)]
    pub sled: Option<with_sled::Args>,
}

impl DbArgs {
    pub fn build(self) -> Result<impl DbInterface, DbInitError> {
        if let Some(args) = &self.sled {
            let db = with_sled::WithSled::try_from(args).expect("oops");
            Ok(db)
        } else {
            Err(DbInitError::None)
        }
    }
}
