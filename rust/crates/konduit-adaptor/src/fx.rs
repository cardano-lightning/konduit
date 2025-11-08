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
    #[clap(long, default_value_t = false)]
    pub with_coin_gecko: bool,

    /// Useful for testing
    #[clap(flatten)]
    pub with_static: Option<with_static::WithStaticArgs>,
}

impl FxArgs {
    pub fn build(self) -> Result<Box<dyn FxInterface>, FxInitError> {
        if self.with_coin_gecko && self.with_static.is_some() {
            return Err(FxInitError::FxError(FxError::Other(
                "Cannot use both: coin gecko and static fx at the same time".to_string(),
            )))?;
        }
        match self.with_static {
            Some(args) => {
                let fx = with_static::WithStatic::try_from(&args).map_err(FxInitError::FxError)?;
                Ok(Box::new(fx))
            }
            None => {
                let coin_gecko_token = std::env::var(crate::env::COIN_GEKO_TOKEN).ok();
                if let Some(token) = coin_gecko_token {
                    let fx = with_coin_gecko::WithCoinGecko::new(Some(token));
                    Ok(Box::new(fx))
                } else if self.with_coin_gecko {
                    let fx = with_coin_gecko::WithCoinGecko::new(None);
                    Ok(Box::new(fx))
                } else {
                    Err(FxInitError::None)
                }
            }
        }
    }
}
