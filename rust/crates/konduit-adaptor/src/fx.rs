use thiserror::Error;

mod interface;
pub use interface::*;

use crate::fx::with_coin_gecko::CoinGeckoArgs;

mod with_coin_gecko;

#[derive(Debug, Error)]
pub enum FxInitError {
    #[error("Fx Error {0}")]
    FxError(FxError),
    #[error("No fx specified")]
    None,
}

#[derive(Debug, Clone, clap::Args)]
pub struct FxArgs {
    /// Db with sled
    #[clap(flatten)]
    pub coin_gecko: Option<CoinGeckoArgs>,
}

impl FxArgs {
    pub fn into(self) -> Result<impl FxInterface, FxInitError> {
        if let Some(args) = self.coin_gecko {
            let fx =
                with_coin_gecko::WithCoinGecko::try_from(args).map_err(FxInitError::FxError)?;
            Ok(fx)
        } else {
            Err(FxInitError::None)
        }
    }
}
