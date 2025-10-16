use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::mixed_cheque::MixedCheque;

#[derive(Debug, Clone)]
pub struct MixedCheques(pub Vec<MixedCheque>);

impl<'a> TryFrom<&PlutusData<'a>> for MixedCheques {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| MixedCheque::try_from(x))
            .collect::<Result<Vec<MixedCheque>>>()?;
        Ok(MixedCheques(inner))
    }
}

impl<'a> From<MixedCheques> for PlutusData<'a> {
    fn from(value: MixedCheques) -> Self {
        Self::list(value.0.into_iter().map(|x| x.into()))
    }
}
