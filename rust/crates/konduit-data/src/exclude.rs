use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::base::Index;

#[derive(Debug, Clone)]
pub struct Exclude(pub Vec<Index>);

impl<'a> TryFrom<&PlutusData<'a>> for Exclude {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list.map(Index::try_from).collect::<Result<Vec<Index>>>()?;
        Ok(Exclude(inner))
    }
}

impl<'a> From<Exclude> for PlutusData<'a> {
    fn from(value: Exclude) -> Self {
        Self::list(value.0.into_iter().map(|x| x.into()))
    }
}
