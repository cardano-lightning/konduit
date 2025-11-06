use async_trait::async_trait;

use crate::fx::interface::{BaseCurrency, Fx, FxError, FxInterface};

#[derive(Debug, Clone, clap::Args)]
pub struct WithStaticArgs {
    /// The path to the database file
    #[clap(long, env = "KONDUIT_FX_BITCOIN")]
    pub bitcoin: f64,
    #[clap(long, env = "KONDUIT_FX_ADA")]
    pub ada: f64,
}

#[derive(Debug, Clone)]
pub struct WithStatic {
    pub bitcoin: f64,
    pub ada: f64,
}

impl TryFrom<&WithStaticArgs> for WithStatic {
    type Error = FxError;

    fn try_from(value: &WithStaticArgs) -> Result<Self, Self::Error> {
        Ok(WithStatic {
            bitcoin: value.bitcoin,
            ada: value.ada,
        })
    }
}

#[async_trait]
impl FxInterface for WithStatic {
    async fn get(&self) -> Result<Fx, FxError> {
        let new: Fx = Fx::new(BaseCurrency::Eur, self.ada, self.bitcoin);
        Ok(new)
    }
}
