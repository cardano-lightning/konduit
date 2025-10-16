use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{
    base::{Amount, Timestamp},
    pendings::Pendings,
};

#[derive(Debug, Clone)]
pub enum Stage {
    Opened(Amount),
    Closed(Amount, Timestamp),
    Responded(Amount, Pendings),
}

impl Stage {
    pub fn new_opened(amount: Amount) -> Self {
        Self::Opened(amount)
    }

    pub fn new_closed(amount: Amount, timeout: Timestamp) -> Self {
        Self::Closed(amount, timeout)
    }

    pub fn new_responded(amount: Amount, pendings: Pendings) -> Self {
        Self::Responded(amount, pendings)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Stage {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_opened(Amount::try_from(a)?))
        } else if xi == 1 {
            let [a, b] = <[PlutusData; 2]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_closed(
                Amount::try_from(a)?,
                Timestamp::try_from(b)?,
            ))
        } else if xi == 2 {
            let [a, b] = <[PlutusData; 2]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_responded(
                Amount::try_from(a)?,
                Pendings::try_from(&b)?,
            ))
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<Stage> for PlutusData<'a> {
    fn from(value: Stage) -> Self {
        match value {
            Stage::Opened(a) => PlutusData::constr(0, vec![PlutusData::from(a)]),
            Stage::Closed(a, b) => {
                PlutusData::constr(1, vec![PlutusData::from(a), PlutusData::from(b)])
            }
            Stage::Responded(a, b) => {
                PlutusData::constr(2, vec![PlutusData::from(a), PlutusData::from(b)])
            }
        }
    }
}
