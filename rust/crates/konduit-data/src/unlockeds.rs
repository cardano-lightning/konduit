use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::unlocked::Unlocked;

#[derive(Debug, Clone)]
pub struct Unlockeds(pub Vec<Unlocked>);

impl<'a> TryFrom<&PlutusData<'a>> for Unlockeds {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| Unlocked::try_from(&x))
            .collect::<Result<Vec<Unlocked>>>()?;
        Ok(Unlockeds(inner))
    }
}

impl<'a> From<Unlockeds> for PlutusData<'a> {
    fn from(value: Unlockeds) -> Self {
        Self::list(value.0.into_iter().map(|x| x.into()))
    }
}
