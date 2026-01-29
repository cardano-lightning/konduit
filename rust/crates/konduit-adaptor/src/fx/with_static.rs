use async_trait::async_trait;

use super::{BaseCurrency, Error, Fx, FxInterface};

#[derive(Debug, Clone, clap::Args)]
pub struct WithStaticArgs {
    /// The path to the database file
    #[clap(long, env = crate::env::FX_BITCOIN)]
    pub bitcoin: f64,
    #[clap(long, env = crate::env::FX_ADA)]
    pub ada: f64,
}

#[derive(Debug, Clone)]
pub struct WithStatic {
    pub bitcoin: f64,
    pub ada: f64,
}

impl TryFrom<&WithStaticArgs> for WithStatic {
    type Error = Error;

    fn try_from(value: &WithStaticArgs) -> Result<Self, Self::Error> {
        Ok(WithStatic {
            bitcoin: value.bitcoin,
            ada: value.ada,
        })
    }
}

#[async_trait]
impl FxInterface for WithStatic {
    async fn get(&self) -> Result<Fx, Error> {
        let new: Fx = Fx::new(BaseCurrency::Eur, self.ada, self.bitcoin);
        Ok(new)
    }
}
