use thiserror::Error;

mod interface;
pub use interface::{BlnError, BlnInterface, PayRequest, PayResponse, QuoteRequest, QuoteResponse};

mod invoice;
pub use invoice::Invoice;

mod with_lnd;

#[derive(Debug, Clone, clap::Args)]
pub struct BlnArgs {
    /// Bln with lnd
    #[clap(flatten)]
    pub lnd: Option<with_lnd::LndArgs>,
}

#[derive(Debug, Error)]
pub enum BlnInitError {
    #[error("LND error : {0}")]
    LndError(BlnError),
    #[error("No BLN specified")]
    None,
}

impl BlnArgs {
    pub fn build(self) -> Result<impl BlnInterface, BlnInitError> {
        if let Some(args) = &self.lnd {
            let db = with_lnd::WithLnd::try_from(args).expect("oops");
            Ok(db)
        } else {
            Err(BlnInitError::None)
        }
    }
}
