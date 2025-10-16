use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::step::Step;

#[derive(Debug, Clone)]
pub struct Steps(pub Vec<Step>);

impl<'a> TryFrom<&PlutusData<'a>> for Steps {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| Step::try_from(x))
            .collect::<Result<Vec<Step>>>()?;
        Ok(Steps(inner))
    }
}

impl<'a> From<Steps> for PlutusData<'a> {
    fn from(value: Steps) -> Self {
        Self::list(value.0.into_iter().map(|x| x.into()))
    }
}
