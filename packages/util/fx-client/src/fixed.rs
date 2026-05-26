use async_trait::async_trait;

use super::{Api, BaseCurrency, State};

#[derive(Debug, Clone)]
pub struct Client {
    pub base: BaseCurrency,
    pub bitcoin: f64,
    pub ada: f64,
}

impl Client {
    pub fn new(base: BaseCurrency, bitcoin: f64, ada: f64) -> Self {
        Self { base, bitcoin, ada }
    }
}

#[async_trait]
impl Api for Client {
    async fn get(&self) -> super::Result<State> {
        let new = State::new(self.base.clone(), self.ada, self.bitcoin);
        Ok(new)
    }
}
