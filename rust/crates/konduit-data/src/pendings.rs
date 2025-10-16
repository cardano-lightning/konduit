use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::pending::Pending;

#[derive(Debug, Clone)]
pub struct Pendings(pub Vec<Pending>);

impl<'a> TryFrom<&PlutusData<'a>> for Pendings {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| Pending::try_from(&x))
            .collect::<Result<Vec<Pending>>>()?;
        Ok(Pendings(inner))
    }
}

impl<'a> From<Pendings> for PlutusData<'a> {
    fn from(value: Pendings) -> Self {
        Self::list(value.0.into_iter().map(|x| x.into()))
    }
}
