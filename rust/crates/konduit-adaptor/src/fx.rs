use thiserror::Error;

mod interface;
pub use interface::*;

mod with_coin_gecko;
mod with_static;

#[derive(Debug, Error)]
pub enum FxInitError {
    #[error("Fx Error {0}")]
    FxError(FxError),
    #[error("No fx specified")]
    None,
}

#[derive(Debug, Clone, clap::Args)]
pub struct FxArgs {
    /// Useful for testing
    #[clap(flatten)]
    pub with_static: Option<with_static::WithStaticArgs>,
    #[clap(flatten)]
    pub coin_gecko: Option<with_coin_gecko::CoinGeckoArgs>,
}

impl FxArgs {
    pub fn build(self) -> Result<Box<dyn FxInterface>, FxInitError> {
        if let Some(args) = &self.with_static {
            let fx = with_static::WithStatic::try_from(args).map_err(FxInitError::FxError)?;
            Ok(Box::new(fx))
        } else if let Some(args) = &self.coin_gecko {
            let fx =
                with_coin_gecko::WithCoinGecko::try_from(args).map_err(FxInitError::FxError)?;
            Ok(Box::new(fx))
        } else {
            Err(FxInitError::None)
        }
    }
}
